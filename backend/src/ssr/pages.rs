use actix_session::Session;
use actix_web::{get, http::header, web, HttpResponse, Responder};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use tera::{Context, Tera};

use crate::entity::{courses as course_entity, modules, users};
use crate::services::auth_helpers::is_enrolled;

#[get("/")]
async fn index(session: Session) -> impl Responder {
    render_page("index.html", &session)
}

#[get("/courses")]
async fn courses(session: Session) -> impl Responder {
    render_page("courses.html", &session)
}

#[get("/lessons")]
async fn lessons(session: Session) -> impl Responder {
    render_page("lessons.html", &session)
}

#[get("/assessments")]
async fn assessments(session: Session) -> impl Responder {
    render_page("assessments.html", &session)
}

#[get("/challenges")]
async fn challenges(session: Session) -> impl Responder {
    render_page("challenges.html", &session)
}

#[get("/certification")]
async fn certification(session: Session) -> impl Responder {
    render_page("certification.html", &session)
}

#[get("/projects")]
async fn projects(session: Session) -> impl Responder {
    render_page("projects.html", &session)
}

#[get("/downloads")]
async fn downloads(session: Session) -> impl Responder {
    render_page("downloads.html", &session)
}

#[get("/course/{course_id}")]
async fn course_details_page(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    let user_id = match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => user_id,
        Ok(None) => {
            return HttpResponse::Found()
                .insert_header((header::LOCATION, "/login"))
                .finish();
        }
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Session error: {}", err));
        }
    };

    let course_id = path.into_inner();
    let course_exists = match course_entity::Entity::find_by_id(course_id)
        .one(db.get_ref())
        .await
    {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding course: {}", err));
        }
    };

    if !course_exists {
        return HttpResponse::NotFound().body("Course not found");
    }

    match user_can_manage_course_content(db.get_ref(), &session, course_id, user_id).await {
        Ok(true) => return render_page("course_details.html", &session),
        Ok(false) => {}
        Err(response) => return response,
    }

    match is_enrolled(db.get_ref(), user_id, course_id).await {
        Ok(true) => render_page("course_details.html", &session),
        Ok(false) => HttpResponse::Forbidden().body("You must be enrolled to view course details"),
        Err(response) => response,
    }
}

#[get("/course/{course_id}/quiz-creator")]
async fn quiz_creator_page(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    let user_id = match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => user_id,
        Ok(None) => {
            return HttpResponse::Found()
                .insert_header((header::LOCATION, "/login"))
                .finish();
        }
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Session error: {}", err));
        }
    };

    let course_id = path.into_inner();
    let course_exists = match course_entity::Entity::find_by_id(course_id)
        .one(db.get_ref())
        .await
    {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding course: {}", err));
        }
    };

    if !course_exists {
        return HttpResponse::NotFound().body("Course not found");
    }

    match user_can_manage_course_content(db.get_ref(), &session, course_id, user_id).await {
        Ok(true) => render_page("quiz_creator.html", &session),
        Ok(false) => HttpResponse::Forbidden().body("You cannot manage quizzes for this course"),
        Err(response) => response,
    }
}

#[get("/pdf-viewer")]
async fn pdf_viewer_page(session: Session) -> impl Responder {
    render_page("pdf_viewer.html", &session)
}

#[get("/module-content/{module_id}")]
async fn module_content_page(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    let user_id = match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => user_id,
        Ok(None) => {
            return HttpResponse::Found()
                .insert_header((header::LOCATION, "/login"))
                .finish();
        }
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Session error: {}", err));
        }
    };

    let module_id = path.into_inner();
    let module = match modules::Entity::find_by_id(module_id).one(db.get_ref()).await {
        Ok(Some(module)) => module,
        Ok(None) => return HttpResponse::NotFound().body("Module not found"),
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding module: {}", err));
        }
    };

    match user_can_manage_course_content(db.get_ref(), &session, module.course_id, user_id).await {
        Ok(true) => return render_page("module_content.html", &session),
        Ok(false) => {}
        Err(response) => return response,
    }

    match is_enrolled(db.get_ref(), user_id, module.course_id).await {
        Ok(true) => render_page("module_content.html", &session),
        Ok(false) => HttpResponse::Forbidden().body("You must be enrolled to view this module"),
        Err(response) => response,
    }
}

async fn user_can_manage_course_content(
    db: &DatabaseConnection,
    session: &Session,
    course_id: i32,
    user_id: i32,
) -> Result<bool, HttpResponse> {
    let role_names: Vec<String> = session
        .get::<Vec<String>>("role_names")
        .ok()
        .flatten()
        .unwrap_or_default();

    if role_names.iter().any(|role| role == "LMS Admin") {
        return Ok(true);
    }

    if !role_names.iter().any(|role| role == "Organisation Admin") {
        return Ok(false);
    }

    let user = users::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding user: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("User not found"))?;

    let course = course_entity::Entity::find()
        .filter(course_entity::Column::CourseId.eq(course_id))
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding course: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Course not found"))?;

    Ok(user.org_id.is_some() && user.org_id == course.org_id)
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(index)
        .service(courses)
        .service(lessons)
        .service(assessments)
        .service(challenges)
        .service(certification)
        .service(projects)
        .service(downloads)
        .service(course_details_page)
        .service(quiz_creator_page)
        .service(module_content_page)
        .service(pdf_viewer_page);
}

pub fn build_page_context(session: &Session) -> Context {
    let mut context: Context = Context::new();

    if let Ok(Some(user_id)) = session.get::<i32>("user_id") {
        context.insert("is_logged_in", &true);
        context.insert("user_id", &user_id);
    } else {
        context.insert("is_logged_in", &false);
    }

    if let Ok(Some(user_email)) = session.get::<String>("user_email") {
        context.insert("user_email", &user_email);
    }

    let email_verified = session
        .get::<bool>("email_verified")
        .ok()
        .flatten()
        .unwrap_or(false);
    context.insert("email_verified", &email_verified);

    let must_change_password = session
        .get::<bool>("must_change_password")
        .ok()
        .flatten()
        .unwrap_or(false);
    context.insert("must_change_password", &must_change_password);

    let role_names: Vec<String> = session
        .get::<Vec<String>>("role_names")
        .ok()
        .flatten()
        .unwrap_or_default();

    context.insert("role_names", &role_names);

    let role_ids: Vec<i32> = session
        .get::<Vec<i32>>("role_ids")
        .ok()
        .flatten()
        .unwrap_or_default();

    context.insert("role_ids", &role_ids);

    context
}

pub fn render_page(template_name: &str, session: &Session) -> HttpResponse {
    if session
        .get::<bool>("must_change_password")
        .ok()
        .flatten()
        .unwrap_or(false)
    {
        return HttpResponse::Found()
            .insert_header((header::LOCATION, "/change-password"))
            .finish();
    }

    let tera: Tera = Tera::new("../frontend/templates/**/*")
        .expect("Failed to load templates");

    let context = build_page_context(session);

    let html: String = tera
        .render(template_name, &context)
        .expect("Failed to render template");

    HttpResponse::Ok()
        .content_type("text/html")
        .body(html)
}
