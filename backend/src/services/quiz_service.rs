use actix_session::Session;
use actix_web::HttpResponse;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::entity::{
    courses,
    quiz::{self, Entity as QuizEntity},
};
use crate::models::quiz::{CreateQuiz, UpdateQuiz};
use crate::services::course_service::can_manage_course;

async fn require_can_manage_course(
    db: &DatabaseConnection,
    session: &Session,
    course_id: i32,
) -> Result<(), HttpResponse> {
    let course = courses::Entity::find_by_id(course_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding course: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Course not found"))?;

    match can_manage_course(db, session, &course).await {
        Ok(true) => Ok(()),
        Ok(false) => {
            Err(HttpResponse::Forbidden().body("You cannot manage quizzes for this course"))
        }
        Err(response) => Err(response),
    }
}

pub async fn list_quizzes(db: &DatabaseConnection) -> HttpResponse {
    match QuizEntity::find().all(db).await {
        Ok(quizzes) if quizzes.is_empty() => HttpResponse::NotFound().body("No quizzes found"),
        Ok(quizzes) => HttpResponse::Ok().json(quizzes),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn list_quizzes_by_course(db: &DatabaseConnection, course_id: i32) -> HttpResponse {
    match QuizEntity::find()
        .filter(quiz::Column::CourseId.eq(course_id))
        .all(db)
        .await
    {
        Ok(quizzes) if quizzes.is_empty() => HttpResponse::NotFound().body("No quizzes found"),
        Ok(quizzes) => HttpResponse::Ok().json(quizzes),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn create_quiz(
    db: &DatabaseConnection,
    session: &Session,
    data: CreateQuiz,
) -> HttpResponse {
    if let Err(response) = require_can_manage_course(db, session, data.course_id).await {
        return response;
    }

    let new_quiz = quiz::ActiveModel {
        course_id: Set(data.course_id),
        title: Set(data.title),
        description: Set(data.description),
        max_attempts: Set(data.max_attempts),
        time_limit: Set(data.time_limit),
        starts_at: Set(data.starts_at),
        ..Default::default()
    };

    match new_quiz.insert(db).await {
        Ok(quiz) => HttpResponse::Ok().json(quiz),
        Err(err) => HttpResponse::InternalServerError().body(format!("Insert error: {}", err)),
    }
}

pub async fn update_quiz(
    db: &DatabaseConnection,
    session: &Session,
    quiz_id: i32,
    data: UpdateQuiz,
) -> HttpResponse {
    match QuizEntity::find_by_id(quiz_id).one(db).await {
        Ok(Some(updated_quiz)) => {
            if let Err(response) =
                require_can_manage_course(db, session, updated_quiz.course_id).await
            {
                return response;
            }

            if let Some(course_id) = data.course_id {
                if course_id != updated_quiz.course_id {
                    return HttpResponse::BadRequest()
                        .body("Moving quizzes between courses is not supported here");
                }
            }

            let mut active: quiz::ActiveModel = updated_quiz.into();

            if let Some(title) = data.title {
                active.title = Set(title);
            }
            if let Some(description) = data.description {
                active.description = Set(Some(description));
            }
            if let Some(max_attempts) = data.max_attempts {
                active.max_attempts = Set(Some(max_attempts));
            }
            if let Some(time_limit) = data.time_limit {
                active.time_limit = Set(Some(time_limit));
            }
            if let Some(starts_at) = data.starts_at {
                active.starts_at = Set(Some(starts_at));
            }

            match active.update(db).await {
                Ok(_) => HttpResponse::Ok().body(format!("Quiz with id {} updated!", quiz_id)),
                Err(err) => {
                    HttpResponse::InternalServerError().body(format!("Update error: {}", err))
                }
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Quiz not found"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn delete_quiz(db: &DatabaseConnection, session: &Session, quiz_id: i32) -> HttpResponse {
    match QuizEntity::find_by_id(quiz_id).one(db).await {
        Ok(Some(target_quiz)) => {
            if let Err(response) =
                require_can_manage_course(db, session, target_quiz.course_id).await
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
