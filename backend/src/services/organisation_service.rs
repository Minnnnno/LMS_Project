use actix_session::Session;
use actix_web::HttpResponse;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set, TransactionTrait,
};

use crate::entity::{
    assignments, course_instructors, courses, enrollments, lesson_progress, module_contents,
    module_prerequisites, module_progress, modules, organisations, payments, quiz, quiz_answers,
    quiz_attempts, quiz_options, quiz_prerequisites, quiz_questions, submissions, users,
};

use crate::services::course_service::has_role;

pub fn require_org_admin(session: &Session) -> Result<(), HttpResponse> {
    if has_role(session, "Organisation Admin") || has_role(session, "LMS Admin") {
        Ok(())
    } else {
        Err(HttpResponse::Forbidden().body("Organisation Admin or LMS Admin role required"))
    }
}

pub async fn delete_organisation_and_dependents(
    db: &DatabaseConnection,
    org_id: i32,
) -> HttpResponse {
    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Delete organisation transaction error: {}", err));
        }
    };

    match organisations::Entity::find_by_id(org_id).one(&txn).await {
        Ok(Some(_)) => {}
        Ok(None) => return HttpResponse::NotFound().body("Organisation not found"),
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Organisation lookup error: {}", err));
        }
    }

    let users_in_org = match users::Entity::find()
        .filter(users::Column::OrgId.eq(org_id))
        .all(&txn)
        .await
    {
        Ok(users) => users,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Organisation user lookup error: {}", err));
        }
    };

    for user in users_in_org {
        let mut active_user = user.into_active_model();
        active_user.org_id = Set(None);

        if let Err(err) = active_user.update(&txn).await {
            return HttpResponse::InternalServerError()
                .body(format!("Organisation user update error: {}", err));
        }
    }

    let course_ids = match courses::Entity::find()
        .filter(courses::Column::OrgId.eq(org_id))
        .all(&txn)
        .await
    {
        Ok(courses) => courses
            .into_iter()
            .map(|course| course.course_id)
            .collect::<Vec<i32>>(),
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Organisation course lookup error: {}", err));
        }
    };

    if !course_ids.is_empty() {
        if let Err(response) = delete_course_dependents(&txn, &course_ids).await {
            return response;
        }

        if let Err(err) = courses::Entity::delete_many()
            .filter(courses::Column::CourseId.is_in(course_ids))
            .exec(&txn)
            .await
        {
            return HttpResponse::InternalServerError()
                .body(format!("Organisation course cleanup error: {}", err));
        }
    }

    match organisations::Entity::delete_by_id(org_id).exec(&txn).await {
        Ok(result) if result.rows_affected > 0 => {}
        Ok(_) => return HttpResponse::NotFound().body("Organisation not found"),
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Delete organisation error: {}", err));
        }
    }

    match txn.commit().await {
        Ok(_) => HttpResponse::Ok().body("Organisation deleted successfully"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Organisation delete commit error: {}", err)),
    }
}

async fn delete_course_dependents<C>(db: &C, course_ids: &[i32]) -> Result<(), HttpResponse>
where
    C: sea_orm::ConnectionTrait,
{
    let module_ids = modules::Entity::find()
        .filter(modules::Column::CourseId.is_in(course_ids.iter().copied()))
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Organisation module lookup error: {}", err))
        })?
        .into_iter()
        .map(|module| module.module_id)
        .collect::<Vec<i32>>();

    let assignment_ids = assignments::Entity::find()
        .filter(assignments::Column::CourseId.is_in(course_ids.iter().copied()))
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Organisation assignment lookup error: {}", err))
        })?
        .into_iter()
        .map(|assignment| assignment.assignment_id)
        .collect::<Vec<i32>>();

    let quiz_ids = quiz::Entity::find()
        .filter(quiz::Column::CourseId.is_in(course_ids.iter().copied()))
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Organisation quiz lookup error: {}", err))
        })?
        .into_iter()
        .map(|quiz| quiz.quiz_id)
        .collect::<Vec<i32>>();

    if !assignment_ids.is_empty() {
        submissions::Entity::delete_many()
            .filter(submissions::Column::AssignmentId.is_in(assignment_ids.iter().copied()))
            .exec(db)
            .await
            .map_err(|err| {
                HttpResponse::InternalServerError()
                    .body(format!("Organisation submission cleanup error: {}", err))
            })?;
    }

    if !quiz_ids.is_empty() {
        delete_quiz_dependents(db, &quiz_ids).await?;
    }

    if !module_ids.is_empty() {
        delete_module_dependents(db, &module_ids).await?;
    }

    payments::Entity::delete_many()
        .filter(payments::Column::CourseId.is_in(course_ids.iter().copied()))
        .exec(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Organisation payment cleanup error: {}", err))
        })?;

    enrollments::Entity::delete_many()
        .filter(enrollments::Column::CourseId.is_in(course_ids.iter().copied()))
        .exec(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Organisation enrollment cleanup error: {}", err))
        })?;

    course_instructors::Entity::delete_many()
        .filter(course_instructors::Column::CourseId.is_in(course_ids.iter().copied()))
        .exec(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!(
                "Organisation course instructor cleanup error: {}",
                err
            ))
        })?;

    assignments::Entity::delete_many()
        .filter(assignments::Column::CourseId.is_in(course_ids.iter().copied()))
        .exec(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Organisation assignment cleanup error: {}", err))
        })?;

    quiz::Entity::delete_many()
        .filter(quiz::Column::CourseId.is_in(course_ids.iter().copied()))
        .exec(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Organisation quiz cleanup error: {}", err))
        })?;

    modules::Entity::delete_many()
        .filter(modules::Column::CourseId.is_in(course_ids.iter().copied()))
        .exec(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Organisation module cleanup error: {}", err))
        })?;

    Ok(())
}

async fn delete_module_dependents<C>(db: &C, module_ids: &[i32]) -> Result<(), HttpResponse>
where
    C: sea_orm::ConnectionTrait,
{
    let module_content_ids = module_contents::Entity::find()
        .filter(module_contents::Column::ModuleId.is_in(module_ids.iter().copied()))
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Organisation module content lookup error: {}", err))
        })?
        .into_iter()
        .map(|content| content.module_content_id)
        .collect::<Vec<i32>>();

    lesson_progress::Entity::delete_many()
        .filter(lesson_progress::Column::LessonId.is_in(module_ids.iter().copied()))
        .exec(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!(
                "Organisation lesson progress cleanup error: {}",
                err
            ))
        })?;

    if !module_content_ids.is_empty() {
        lesson_progress::Entity::delete_many()
            .filter(lesson_progress::Column::LessonId.is_in(module_content_ids.iter().copied()))
            .exec(db)
            .await
            .map_err(|err| {
                HttpResponse::InternalServerError().body(format!(
                    "Organisation content lesson progress cleanup error: {}",
                    err
                ))
            })?;
    }

    module_progress::Entity::delete_many()
        .filter(module_progress::Column::ModuleId.is_in(module_ids.iter().copied()))
        .exec(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!(
                "Organisation module progress cleanup error: {}",
                err
            ))
        })?;

    module_contents::Entity::delete_many()
        .filter(module_contents::Column::ModuleId.is_in(module_ids.iter().copied()))
        .exec(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!(
                "Organisation module content cleanup error: {}",
                err
            ))
        })?;

    module_prerequisites::Entity::delete_many()
        .filter(module_prerequisites::Column::ModuleId.is_in(module_ids.iter().copied()))
        .exec(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!(
                "Organisation module prerequisite cleanup error: {}",
                err
            ))
        })?;

    module_prerequisites::Entity::delete_many()
        .filter(module_prerequisites::Column::RequiredModuleId.is_in(module_ids.iter().copied()))
        .exec(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!(
                "Organisation required module cleanup error: {}",
                err
            ))
        })?;

    quiz_prerequisites::Entity::delete_many()
        .filter(quiz_prerequisites::Column::RequiredModuleId.is_in(module_ids.iter().copied()))
        .exec(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!(
                "Organisation quiz prerequisite module cleanup error: {}",
                err
            ))
        })?;

    Ok(())
}

async fn delete_quiz_dependents<C>(db: &C, quiz_ids: &[i32]) -> Result<(), HttpResponse>
where
    C: sea_orm::ConnectionTrait,
{
    let question_ids = quiz_questions::Entity::find()
        .filter(quiz_questions::Column::QuizId.is_in(quiz_ids.iter().copied()))
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Organisation quiz question lookup error: {}", err))
        })?
        .into_iter()
        .map(|question| question.question_id)
        .collect::<Vec<i32>>();

    let attempt_ids = quiz_attempts::Entity::find()
        .filter(quiz_attempts::Column::QuizId.is_in(quiz_ids.iter().copied()))
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Organisation quiz attempt lookup error: {}", err))
        })?
        .into_iter()
        .map(|attempt| attempt.attempt_id)
        .collect::<Vec<i32>>();

    if !attempt_ids.is_empty() {
        quiz_answers::Entity::delete_many()
            .filter(quiz_answers::Column::AttemptId.is_in(attempt_ids.iter().copied()))
            .exec(db)
            .await
            .map_err(|err| {
                HttpResponse::InternalServerError().body(format!(
                    "Organisation quiz attempt answer cleanup error: {}",
                    err
                ))
            })?;
    }

    if !question_ids.is_empty() {
        quiz_answers::Entity::delete_many()
            .filter(quiz_answers::Column::QuestionId.is_in(question_ids.iter().copied()))
            .exec(db)
            .await
            .map_err(|err| {
                HttpResponse::InternalServerError()
                    .body(format!("Organisation quiz answer cleanup error: {}", err))
            })?;

        quiz_options::Entity::delete_many()
            .filter(quiz_options::Column::QuestionId.is_in(question_ids.iter().copied()))
            .exec(db)
            .await
            .map_err(|err| {
                HttpResponse::InternalServerError()
                    .body(format!("Organisation quiz option cleanup error: {}", err))
            })?;
    }

    quiz_prerequisites::Entity::delete_many()
        .filter(quiz_prerequisites::Column::QuizId.is_in(quiz_ids.iter().copied()))
        .exec(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!(
                "Organisation quiz prerequisite cleanup error: {}",
                err
            ))
        })?;

    quiz_attempts::Entity::delete_many()
        .filter(quiz_attempts::Column::QuizId.is_in(quiz_ids.iter().copied()))
        .exec(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Organisation quiz attempt cleanup error: {}", err))
        })?;

    quiz_questions::Entity::delete_many()
        .filter(quiz_questions::Column::QuizId.is_in(quiz_ids.iter().copied()))
        .exec(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Organisation quiz question cleanup error: {}", err))
        })?;

    Ok(())
}
