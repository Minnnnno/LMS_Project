use actix_session::Session;
use actix_web::{
    HttpRequest, HttpResponse, Responder, get,
    http::{StatusCode, header},
    web,
};
use sea_orm::{DatabaseConnection, EntityTrait};
use tera::{Context, Tera};

use crate::entity::{courses as course_entity, modules, quiz};
use crate::services::auth_helpers::is_enrolled;
use crate::services::course_service::{can_manage_course, has_role};

#[get("/")]
async fn index(session: Session) -> impl Responder {
    if session.get::<i32>("user_id").ok().flatten().is_none() {
        return HttpResponse::Found()
            .insert_header((header::LOCATION, "/courses"))
            .finish();
    }

    render_page("index.html", &session)
}

#[get("/courses")]
async fn courses(session: Session) -> impl Responder {
    render_page("courses.html", &session)
}

#[get("/features")]
async fn features(session: Session) -> impl Responder {
    render_page("features.html", &session)
}

#[get("/about")]
async fn about(session: Session) -> impl Responder {
    render_page("about.html", &session)
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

#[get("/verify/certificate/{token}")]
async fn certificate_verification(session: Session) -> impl Responder {
    render_page("certificate_verification.html", &session)
}

#[get("/projects")]
async fn projects(session: Session) -> impl Responder {
    render_page("projects.html", &session)
}

#[get("/downloads")]
async fn downloads(session: Session) -> impl Responder {
    render_page("downloads.html", &session)
}

#[get("/instructor/submissions")]
async fn instructor_submissions(session: Session) -> impl Responder {
    if session.get::<i32>("user_id").ok().flatten().is_none() {
        return HttpResponse::Found()
            .insert_header((header::LOCATION, "/login"))
            .finish();
    }

    if !has_role(&session, "Instructor")
        && !has_role(&session, "Organisation Admin")
        && !has_role(&session, "LMS Admin")
    {
        return redirect_to_home();
    }

    render_page("instructor_submissions.html", &session)
}

fn redirect_to_home() -> HttpResponse {
    HttpResponse::Found()
        .insert_header((header::LOCATION, "/"))
        .finish()
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
            return HttpResponse::InternalServerError().body(format!("Session error: {}", err));
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
        Err(response) if response.status() == StatusCode::FORBIDDEN => return redirect_to_home(),
        Err(response) => return response,
    }

    match is_enrolled(db.get_ref(), user_id, course_id).await {
        Ok(true) => render_page("course_details.html", &session),
        Ok(false) => redirect_to_home(),
        Err(response) => response,
    }
}

#[get("/course/{course_id}/quiz-builder")]
async fn quiz_builder_page(
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
            return HttpResponse::InternalServerError().body(format!("Session error: {}", err));
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
        Ok(true) => render_page("quiz_builder.html", &session),
        Ok(false) => redirect_to_home(),
        Err(response) if response.status() == StatusCode::FORBIDDEN => redirect_to_home(),
        Err(response) => response,
    }
}

#[get("/course/{course_id}/quiz/{quiz_id}/attempt")]
async fn quiz_attempt_page(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<(i32, i32)>,
) -> impl Responder {
    let user_id = match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => user_id,
        Ok(None) => {
            return HttpResponse::Found()
                .insert_header((header::LOCATION, "/login"))
                .finish();
        }
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Session error: {}", err));
        }
    };

    let (course_id, quiz_id) = path.into_inner();
    let quiz = match quiz::Entity::find_by_id(quiz_id).one(db.get_ref()).await {
        Ok(Some(quiz)) => quiz,
        Ok(None) => return HttpResponse::NotFound().body("Quiz not found"),
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding quiz: {}", err));
        }
    };

    if quiz.course_id != course_id {
        return HttpResponse::NotFound().body("Quiz not found for this course");
    }

    match user_can_manage_course_content(db.get_ref(), &session, course_id, user_id).await {
        Ok(true) => return render_page("quiz_attempt.html", &session),
        Ok(false) => {}
        Err(response) if response.status() == StatusCode::FORBIDDEN => return redirect_to_home(),
        Err(response) => return response,
    }

    match is_enrolled(db.get_ref(), user_id, course_id).await {
        Ok(true) => render_page("quiz_attempt.html", &session),
        Ok(false) => redirect_to_home(),
        Err(response) => response,
    }
}

#[get("/pdf-viewer")]
async fn pdf_viewer_page(session: Session) -> impl Responder {
    render_page("pdf_viewer.html", &session)
}

pub async fn not_found_page(req: HttpRequest, session: Session) -> impl Responder {
    if req.path() == "/api" || req.path().starts_with("/api/") {
        return HttpResponse::NotFound().body("Not found");
    }

    render_error_page("404.html", StatusCode::NOT_FOUND, Some(&session))
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
            return HttpResponse::InternalServerError().body(format!("Session error: {}", err));
        }
    };

    let module_id = path.into_inner();
    let module = match modules::Entity::find_by_id(module_id)
        .one(db.get_ref())
        .await
    {
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
        Err(response) if response.status() == StatusCode::FORBIDDEN => return redirect_to_home(),
        Err(response) => return response,
    }

    match is_enrolled(db.get_ref(), user_id, module.course_id).await {
        Ok(true) => render_page("module_content.html", &session),
        Ok(false) => redirect_to_home(),
        Err(response) => response,
    }
}

async fn user_can_manage_course_content(
    db: &DatabaseConnection,
    session: &Session,
    course_id: i32,
    _user_id: i32,
) -> Result<bool, HttpResponse> {
    let course = course_entity::Entity::find_by_id(course_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding course: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Course not found"))?;

    can_manage_course(db, session, &course).await
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(index)
        .service(courses)
        .service(features)
        .service(about)
        .service(lessons)
        .service(assessments)
        .service(challenges)
        .service(certification)
        .service(certificate_verification)
        .service(projects)
        .service(downloads)
        .service(instructor_submissions)
        .service(course_details_page)
        .service(quiz_builder_page)
        .service(quiz_attempt_page)
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

    let is_instructor_managed_only = role_names.iter().any(|role| role == "Instructor")
        && !role_names.iter().any(|role| role == "Organisation Admin")
        && !role_names.iter().any(|role| role == "LMS Admin");
    context.insert("is_instructor_managed_only", &is_instructor_managed_only);

    let role_ids: Vec<i32> = session
        .get::<Vec<i32>>("role_ids")
        .ok()
        .flatten()
        .unwrap_or_default();

    context.insert("role_ids", &role_ids);

    context
}

fn build_anonymous_page_context() -> Context {
    let mut context = Context::new();
    let role_names: Vec<String> = Vec::new();
    let role_ids: Vec<i32> = Vec::new();

    context.insert("is_logged_in", &false);
    context.insert("email_verified", &false);
    context.insert("must_change_password", &false);
    context.insert("role_names", &role_names);
    context.insert("role_ids", &role_ids);
    context.insert("is_instructor_managed_only", &false);
    context
}

fn render_template_response(
    template_name: &str,
    context: &Context,
    status: StatusCode,
    fallback_message: &'static str,
) -> HttpResponse {
    let fallback_status = if status == StatusCode::OK {
        StatusCode::INTERNAL_SERVER_ERROR
    } else {
        status
    };

    let tera = match Tera::new("../frontend/templates/**/*") {
        Ok(tera) => tera,
        Err(_) => {
            return HttpResponse::build(fallback_status)
                .content_type("text/plain")
                .body(fallback_message);
        }
    };

    match tera.render(template_name, context) {
        Ok(html) => HttpResponse::build(status)
            .content_type("text/html")
            .body(html),
        Err(_) => HttpResponse::build(fallback_status)
            .content_type("text/plain")
            .body(fallback_message),
    }
}

pub fn render_error_page(
    template_name: &str,
    status: StatusCode,
    session: Option<&Session>,
) -> HttpResponse {
    let context = session
        .map(build_page_context)
        .unwrap_or_else(build_anonymous_page_context);

    render_template_response(
        template_name,
        &context,
        status,
        "pspspsps something went wrong on our end",
    )
}

pub fn render_page(template_name: &str, session: &Session) -> HttpResponse {
    render_page_with_status(template_name, session, actix_web::http::StatusCode::OK)
}

pub fn render_page_with_status(
    template_name: &str,
    session: &Session,
    status: actix_web::http::StatusCode,
) -> HttpResponse {
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

    let context = build_page_context(session);

    render_template_response(
        template_name,
        &context,
        status,
        "pspspsps something went wrong on our end",
    )
}
