use std::collections::HashMap;

use actix_session::Session;
use actix_web::HttpResponse;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

use crate::entity::{quiz_answers, quiz_attempts, quiz_options, quiz_questions, users};
use crate::models::quiz_attempts::{
    QuizAttemptReviewAnswer, StaffQuizAttempt, StudentQuizAttemptReview,
};
use crate::services::auth_helpers::get_user_id;
use crate::services::quiz_helper;

pub async fn list_staff_attempts(
    db: &DatabaseConnection,
    session: &Session,
    quiz_id: i32,
) -> HttpResponse {
    if let Err(response) = quiz_helper::require_can_manage_quiz(db, session, quiz_id).await {
        return response;
    }

    let questions = match quiz_questions::Entity::find()
        .filter(quiz_questions::Column::QuizId.eq(quiz_id))
        .order_by_asc(quiz_questions::Column::Position)
        .all(db)
        .await
    {
        Ok(questions) => questions,
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Database error: {}", err));
        }
    };
    let attempts = match quiz_attempts::Entity::find()
        .filter(quiz_attempts::Column::QuizId.eq(quiz_id))
        .order_by_desc(quiz_attempts::Column::StartedAt)
        .all(db)
        .await
    {
        Ok(attempts) => attempts,
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Database error: {}", err));
        }
    };
    let attempt_ids = attempts
        .iter()
        .map(|attempt| attempt.attempt_id)
        .collect::<Vec<_>>();
    let user_ids = attempts
        .iter()
        .map(|attempt| attempt.user_id)
        .collect::<Vec<_>>();
    let question_ids = questions
        .iter()
        .map(|question| question.question_id)
        .collect::<Vec<_>>();

    let students = if user_ids.is_empty() {
        Vec::new()
    } else {
        match users::Entity::find()
            .filter(users::Column::UserId.is_in(user_ids))
            .all(db)
            .await
        {
            Ok(students) => students,
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Database error: {}", err));
            }
        }
    };
    let answers = if attempt_ids.is_empty() {
        Vec::new()
    } else {
        match quiz_answers::Entity::find()
            .filter(quiz_answers::Column::AttemptId.is_in(attempt_ids))
            .all(db)
            .await
        {
            Ok(answers) => answers,
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Database error: {}", err));
            }
        }
    };
    let options = if question_ids.is_empty() {
        Vec::new()
    } else {
        match quiz_options::Entity::find()
            .filter(quiz_options::Column::QuestionId.is_in(question_ids))
            .all(db)
            .await
        {
            Ok(options) => options,
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Database error: {}", err));
            }
        }
    };

    let students_by_id = students
        .into_iter()
        .map(|user| (user.user_id, user))
        .collect::<HashMap<_, _>>();
    let answers_by_key = answers
        .into_iter()
        .map(|answer| ((answer.attempt_id, answer.question_id), answer))
        .collect::<HashMap<_, _>>();
    let options_by_id = options
        .iter()
        .cloned()
        .map(|option| (option.option_id, option))
        .collect::<HashMap<_, _>>();
    let correct_by_question = options
        .into_iter()
        .filter(|option| option.is_correct)
        .map(|option| (option.question_id, option))
        .collect::<HashMap<_, _>>();
    let max_score = questions.iter().map(|question| question.points).sum();

    let payload = attempts
        .into_iter()
        .filter_map(|attempt| {
            let student = students_by_id.get(&attempt.user_id)?;
            let review_answers = questions
                .iter()
                .map(|question| {
                    let answer = answers_by_key.get(&(attempt.attempt_id, question.question_id));
                    let selected = answer
                        .and_then(|answer| answer.selected_option_id)
                        .and_then(|option_id| options_by_id.get(&option_id));
                    let correct = correct_by_question.get(&question.question_id);
                    QuizAttemptReviewAnswer {
                        answer_id: answer.map(|answer| answer.answer_id),
                        question_id: question.question_id,
                        question_type: question.question_type.clone(),
                        question_text: question.question_text.clone(),
                        points: question.points,
                        selected_option_id: selected.map(|option| option.option_id),
                        selected_option_text: selected.map(|option| option.option_text.clone()),
                        correct_option_id: correct.map(|option| option.option_id),
                        correct_option_text: correct.map(|option| option.option_text.clone()),
                        answer_text: answer.and_then(|answer| answer.answer_text.clone()),
                        score: answer.and_then(|answer| answer.score),
                        feedback: answer.and_then(|answer| answer.feedback.clone()),
                    }
                })
                .collect();
            Some(StaffQuizAttempt {
                attempt_id: attempt.attempt_id,
                quiz_id: attempt.quiz_id,
                user_id: attempt.user_id,
                student_name: format!("{} {}", student.first_name, student.last_name)
                    .trim()
                    .to_string(),
                student_email: student.email.clone(),
                started_at: attempt.started_at,
                submitted_at: attempt.submitted_at,
                total_score: attempt.total_score,
                max_score,
                is_graded: attempt.is_graded,
                answers: review_answers,
            })
        })
        .collect::<Vec<_>>();

    HttpResponse::Ok().json(payload)
}

pub async fn get_student_review(
    db: &DatabaseConnection,
    session: &Session,
    attempt_id: i32,
) -> HttpResponse {
    let user_id = match get_user_id(session) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };
    let attempt = match quiz_attempts::Entity::find_by_id(attempt_id).one(db).await {
        Ok(Some(attempt)) => attempt,
        Ok(None) => return HttpResponse::NotFound().body("Attempt not found"),
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Database error: {}", err));
        }
    };
    if attempt.user_id != user_id {
        return HttpResponse::Forbidden().body("You can only view your own quiz attempt");
    }
    if !attempt.is_graded {
        return HttpResponse::Forbidden().body("This quiz attempt has not been graded yet");
    }

    let questions = match quiz_questions::Entity::find()
        .filter(quiz_questions::Column::QuizId.eq(attempt.quiz_id))
        .order_by_asc(quiz_questions::Column::Position)
        .all(db)
        .await
    {
        Ok(questions) => questions,
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Database error: {}", err));
        }
    };
    let answers = match quiz_answers::Entity::find()
        .filter(quiz_answers::Column::AttemptId.eq(attempt_id))
        .all(db)
        .await
    {
        Ok(answers) => answers,
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Database error: {}", err));
        }
    };
    let question_ids = questions
        .iter()
        .map(|question| question.question_id)
        .collect::<Vec<_>>();
    let options = if question_ids.is_empty() {
        Vec::new()
    } else {
        match quiz_options::Entity::find()
            .filter(quiz_options::Column::QuestionId.is_in(question_ids))
            .all(db)
            .await
        {
            Ok(options) => options,
            Err(err) => {
                return HttpResponse::InternalServerError().body(format!("Database error: {}", err));
            }
        }
    };
    let answers_by_question = answers
        .into_iter()
        .map(|answer| (answer.question_id, answer))
        .collect::<HashMap<_, _>>();
    let options_by_id = options
        .iter()
        .cloned()
        .map(|option| (option.option_id, option))
        .collect::<HashMap<_, _>>();
    let correct_by_question = options
        .into_iter()
        .filter(|option| option.is_correct)
        .map(|option| (option.question_id, option))
        .collect::<HashMap<_, _>>();
    let max_score = questions.iter().map(|question| question.points).sum();
    let review_answers = questions
        .into_iter()
        .map(|question| {
            let answer = answers_by_question.get(&question.question_id);
            let selected = answer
                .and_then(|answer| answer.selected_option_id)
                .and_then(|option_id| options_by_id.get(&option_id));
            let correct = correct_by_question.get(&question.question_id);
            QuizAttemptReviewAnswer {
                answer_id: answer.map(|answer| answer.answer_id),
                question_id: question.question_id,
                question_type: question.question_type,
                question_text: question.question_text,
                points: question.points,
                selected_option_id: selected.map(|option| option.option_id),
                selected_option_text: selected.map(|option| option.option_text.clone()),
                correct_option_id: correct.map(|option| option.option_id),
                correct_option_text: correct.map(|option| option.option_text.clone()),
                answer_text: answer.and_then(|answer| answer.answer_text.clone()),
                score: answer.and_then(|answer| answer.score),
                feedback: answer.and_then(|answer| answer.feedback.clone()),
            }
        })
        .collect();

    HttpResponse::Ok().json(StudentQuizAttemptReview {
        attempt_id,
        quiz_id: attempt.quiz_id,
        total_score: attempt.total_score,
        max_score,
        submitted_at: attempt.submitted_at,
        answers: review_answers,
    })
}
