use actix_session::Session;
use actix_web::HttpResponse;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};
use serde::Serialize;
use std::collections::HashSet;

use crate::entity::quiz::{self, Entity as QuizEntity};
use crate::entity::quiz_attempts::{Column as QuizAttemptColumn, Entity as QuizAttemptEntity};
use crate::entity::quiz_options::ActiveModel as QuizOptionActiveModel;
use crate::entity::quiz_questions::{
    ActiveModel as QuizQuestionActiveModel, Column as QuizQuestionColumn,
    Entity as QuizQuestionEntity, QuestionType,
};
use crate::models::quiz::{QuizEditorPayload, SaveQuizDraft};
use crate::services::prerequisite_service;
use crate::services::quiz_helper;

fn validate_quiz_fields(
    title: Option<&str>,
    max_attempts: Option<i32>,
    time_limit: Option<i32>,
) -> Result<(), HttpResponse> {
    if title.map(|value| value.trim().is_empty()).unwrap_or(false) {
        return Err(HttpResponse::BadRequest().body("Quiz title cannot be empty"));
    }

    if max_attempts.map(|value| value < 1).unwrap_or(false) {
        return Err(HttpResponse::BadRequest().body("Max attempts must be 1 or higher"));
    }

    if time_limit.map(|value| value < 1).unwrap_or(false) {
        return Err(HttpResponse::BadRequest().body("Time limit must be 1 minute or higher"));
    }

    Ok(())
}

fn validate_quiz_draft(data: &SaveQuizDraft) -> Result<(), HttpResponse> {
    validate_quiz_fields(Some(&data.title), data.max_attempts, data.time_limit)?;
    if data.questions.is_empty() {
        return Err(HttpResponse::BadRequest().body("A quiz must contain at least one question"));
    }

    let mut question_positions = HashSet::new();
    for question in &data.questions {
        if question.question_text.trim().is_empty() {
            return Err(HttpResponse::BadRequest().body("Question text cannot be empty"));
        }
        if question.position < 1 || !question_positions.insert(question.position) {
            return Err(HttpResponse::BadRequest()
                .body("Question positions must be unique and 1 or higher"));
        }
        if question.points < 1 {
            return Err(HttpResponse::BadRequest().body("Question points must be 1 or higher"));
        }

        match question.question_type {
            QuestionType::Mcq => {
                if question.options.len() < 2 {
                    return Err(HttpResponse::BadRequest()
                        .body("Each MCQ must contain at least two options"));
                }
                if question
                    .options
                    .iter()
                    .filter(|option| option.is_correct)
                    .count()
                    != 1
                {
                    return Err(HttpResponse::BadRequest()
                        .body("Each MCQ must contain exactly one correct option"));
                }
                let mut option_positions = HashSet::new();
                for option in &question.options {
                    if option.option_text.trim().is_empty() {
                        return Err(HttpResponse::BadRequest().body("Option text cannot be empty"));
                    }
                    if option.position < 1 || !option_positions.insert(option.position) {
                        return Err(HttpResponse::BadRequest()
                            .body("Option positions must be unique and 1 or higher"));
                    }
                }
            }
            QuestionType::LongAnswer if !question.options.is_empty() => {
                return Err(
                    HttpResponse::BadRequest().body("Written questions cannot contain MCQ options")
                );
            }
            QuestionType::LongAnswer => {}
        }
    }

    Ok(())
}

pub async fn save_quiz_draft(
    db: &DatabaseConnection,
    session: &Session,
    quiz_id: Option<i32>,
    data: SaveQuizDraft,
) -> HttpResponse {
    if let Err(response) = validate_quiz_draft(&data) {
        return response;
    }

    let existing_quiz = if let Some(quiz_id) = quiz_id {
        match QuizEntity::find_by_id(quiz_id).one(db).await {
            Ok(Some(quiz)) => {
                if quiz.course_id != data.course_id {
                    return HttpResponse::BadRequest()
                        .body("Moving quizzes between courses is not supported");
                }
                if let Err(response) =
                    quiz_helper::require_can_manage_course_id(db, session, quiz.course_id).await
                {
                    return response;
                }
                if let Err(response) = quiz_helper::ensure_content_editable(db, quiz_id).await {
                    return response;
                }
                Some(quiz)
            }
            Ok(None) => return HttpResponse::NotFound().body("Quiz not found"),
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Database error: {}", err));
            }
        }
    } else {
        if let Err(response) =
            quiz_helper::require_can_manage_course_id(db, session, data.course_id).await
        {
            return response;
        }
        None
    };

    let transaction = match db.begin().await {
        Ok(transaction) => transaction,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Could not start quiz save: {}", err));
        }
    };

    if let Some(existing_quiz) = existing_quiz.as_ref()
        && let Err(response) = quiz_helper::lock_quiz(&transaction, existing_quiz.quiz_id).await
    {
        let _ = transaction.rollback().await;
        return response;
    }

    let saved_quiz = if let Some(existing_quiz) = existing_quiz {
        match QuizAttemptEntity::find()
            .filter(QuizAttemptColumn::QuizId.eq(existing_quiz.quiz_id))
            .one(&transaction)
            .await
        {
            Ok(Some(_)) => {
                let _ = transaction.rollback().await;
                return HttpResponse::Conflict()
                    .body("Quiz content cannot be changed after attempts have started");
            }
            Ok(None) => {}
            Err(err) => {
                let _ = transaction.rollback().await;
                return HttpResponse::InternalServerError()
                    .body(format!("Database error checking quiz attempts: {}", err));
            }
        }

        let quiz_id = existing_quiz.quiz_id;
        let mut active: quiz::ActiveModel = existing_quiz.into();
        active.title = Set(data.title.clone());
        active.description = Set(data.description.clone());
        active.max_attempts = Set(data.max_attempts);
        active.time_limit = Set(data.time_limit);
        active.starts_at = Set(data.starts_at);
        let saved = match active.update(&transaction).await {
            Ok(saved) => saved,
            Err(err) => {
                let _ = transaction.rollback().await;
                return HttpResponse::InternalServerError()
                    .body(format!("Quiz update error: {}", err));
            }
        };
        if let Err(err) = QuizQuestionEntity::delete_many()
            .filter(QuizQuestionColumn::QuizId.eq(quiz_id))
            .exec(&transaction)
            .await
        {
            let _ = transaction.rollback().await;
            return HttpResponse::InternalServerError()
                .body(format!("Question replacement error: {}", err));
        }
        saved
    } else {
        match (quiz::ActiveModel {
            course_id: Set(data.course_id),
            title: Set(data.title.clone()),
            description: Set(data.description.clone()),
            max_attempts: Set(data.max_attempts),
            time_limit: Set(data.time_limit),
            starts_at: Set(data.starts_at),
            ..Default::default()
        })
        .insert(&transaction)
        .await
        {
            Ok(saved) => saved,
            Err(err) => {
                let _ = transaction.rollback().await;
                return HttpResponse::InternalServerError()
                    .body(format!("Quiz insert error: {}", err));
            }
        }
    };

    if let Err(response) = prerequisite_service::replace_quiz_prerequisites(
        &transaction,
        saved_quiz.course_id,
        saved_quiz.quiz_id,
        data.prerequisite_module_ids,
    )
    .await
    {
        let _ = transaction.rollback().await;
        return response;
    }

    for question in data.questions {
        let saved_question = match (QuizQuestionActiveModel {
            quiz_id: Set(saved_quiz.quiz_id),
            question_type: Set(question.question_type),
            question_text: Set(question.question_text),
            position: Set(question.position),
            points: Set(question.points),
            ..Default::default()
        })
        .insert(&transaction)
        .await
        {
            Ok(saved) => saved,
            Err(err) => {
                let _ = transaction.rollback().await;
                return HttpResponse::InternalServerError()
                    .body(format!("Question insert error: {}", err));
            }
        };

        for option in question.options {
            if let Err(err) = (QuizOptionActiveModel {
                question_id: Set(saved_question.question_id),
                option_text: Set(option.option_text),
                is_correct: Set(option.is_correct),
                position: Set(option.position),
                ..Default::default()
            })
            .insert(&transaction)
            .await
            {
                let _ = transaction.rollback().await;
                return HttpResponse::InternalServerError()
                    .body(format!("Option insert error: {}", err));
            }
        }
    }

    if let Err(err) = transaction.commit().await {
        return HttpResponse::InternalServerError().body(format!("Quiz save error: {}", err));
    }

    HttpResponse::Ok().json(saved_quiz)
}

pub async fn get_quiz_editor(
    db: &DatabaseConnection,
    session: &Session,
    quiz_id: i32,
) -> HttpResponse {
    let quiz = match quiz_helper::require_can_manage_quiz(db, session, quiz_id).await {
        Ok(quiz) => quiz,
        Err(response) => return response,
    };
    let questions = match quiz_helper::load_editor_questions(db, quiz_id).await {
        Ok(questions) => questions,
        Err(response) => return response,
    };
    let prerequisite_module_ids =
        match prerequisite_service::get_quiz_prerequisite_ids(db, quiz_id).await {
            Ok(ids) => ids,
            Err(response) => return response,
        };

    HttpResponse::Ok().json(QuizEditorPayload {
        quiz_id,
        course_id: quiz.course_id,
        title: quiz.title,
        description: quiz.description,
        max_attempts: quiz.max_attempts,
        time_limit: quiz.time_limit,
        starts_at: quiz.starts_at,
        prerequisite_module_ids,
        questions,
    })
}

#[derive(Serialize)]
struct QuizPayload {
    quiz_id: i32,
    course_id: i32,
    title: String,
    description: Option<String>,
    max_attempts: Option<i32>,
    time_limit: Option<i32>,
    starts_at: Option<chrono::NaiveDateTime>,
    ends_at: Option<chrono::NaiveDateTime>,
    created_at: chrono::NaiveDateTime,
    prerequisite_module_ids: Vec<i32>,
}

async fn quiz_payloads(
    db: &DatabaseConnection,
    quizzes: Vec<quiz::Model>,
) -> Result<Vec<QuizPayload>, HttpResponse> {
    let mut payloads = Vec::with_capacity(quizzes.len());

    for quiz in quizzes {
        let prerequisite_module_ids =
            prerequisite_service::get_quiz_prerequisite_ids(db, quiz.quiz_id).await?;

        payloads.push(QuizPayload {
            quiz_id: quiz.quiz_id,
            course_id: quiz.course_id,
            title: quiz.title,
            description: quiz.description,
            max_attempts: quiz.max_attempts,
            time_limit: quiz.time_limit,
            starts_at: quiz.starts_at,
            ends_at: quiz.ends_at,
            created_at: quiz.created_at,
            prerequisite_module_ids,
        });
    }

    Ok(payloads)
}

pub async fn list_quizzes_by_course(db: &DatabaseConnection, course_id: i32) -> HttpResponse {
    match QuizEntity::find()
        .filter(quiz::Column::CourseId.eq(course_id))
        .all(db)
        .await
    {
        Ok(quizzes) if quizzes.is_empty() => HttpResponse::NotFound().body("No quizzes found"),
        Ok(quizzes) => match quiz_payloads(db, quizzes).await {
            Ok(payloads) => HttpResponse::Ok().json(payloads),
            Err(response) => response,
        },
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn delete_quiz(db: &DatabaseConnection, session: &Session, quiz_id: i32) -> HttpResponse {
    match QuizEntity::find_by_id(quiz_id).one(db).await {
        Ok(Some(target_quiz)) => {
            if let Err(response) =
                quiz_helper::require_can_manage_course_id(db, session, target_quiz.course_id).await
            {
                return response;
            }

            let active_model: quiz::ActiveModel = target_quiz.into();
            match active_model.delete(db).await {
                Ok(_) => HttpResponse::Ok().body("Quiz deleted!"),
                Err(err) => {
                    HttpResponse::InternalServerError().body(format!("Delete error: {}", err))
                }
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Quiz not found!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Delete error {}", err)),
    }
}

#[cfg(test)]
mod tests {
    use super::validate_quiz_draft;
    use crate::entity::quiz_questions::QuestionType;
    use crate::models::quiz::{SaveQuizDraft, SaveQuizOption, SaveQuizQuestion};

    fn valid_draft() -> SaveQuizDraft {
        SaveQuizDraft {
            course_id: 1,
            title: "Quiz".to_string(),
            description: None,
            max_attempts: Some(1),
            time_limit: Some(30),
            starts_at: None,
            prerequisite_module_ids: Vec::new(),
            questions: vec![SaveQuizQuestion {
                question_type: QuestionType::Mcq,
                question_text: "Question".to_string(),
                position: 1,
                points: 1,
                options: vec![
                    SaveQuizOption {
                        option_text: "Correct".to_string(),
                        is_correct: true,
                        position: 1,
                    },
                    SaveQuizOption {
                        option_text: "Wrong".to_string(),
                        is_correct: false,
                        position: 2,
                    },
                ],
            }],
        }
    }

    #[test]
    fn accepts_a_valid_aggregate_quiz() {
        assert!(validate_quiz_draft(&valid_draft()).is_ok());
    }

    #[test]
    fn rejects_duplicate_question_positions() {
        let mut draft = valid_draft();
        draft.questions.push(SaveQuizQuestion {
            question_type: QuestionType::LongAnswer,
            question_text: "Written".to_string(),
            position: 1,
            points: 2,
            options: Vec::new(),
        });

        assert!(validate_quiz_draft(&draft).is_err());
    }

    #[test]
    fn rejects_multiple_correct_mcq_options() {
        let mut draft = valid_draft();
        draft.questions[0].options[1].is_correct = true;

        assert!(validate_quiz_draft(&draft).is_err());
    }
}
