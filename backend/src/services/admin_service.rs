use actix_web::HttpResponse;
use chrono::{DateTime, Datelike, FixedOffset, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    IntoActiveModel, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde::Serialize;
use serde_json::json;
use std::collections::{BTreeMap, HashMap, HashSet};
use validator::Validate;

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};

use crate::entity::courses::CourseStatus;
use crate::entity::{
    courses, enrollments, organisation_signup_requests, organisations, payments, roles, user_roles,
    users,
};

use crate::models::admin::{
    AdminEnrollmentForm, CreateAdminCourseForm, CreateAdminUserForm, CreateOrganisationForm,
    RejectOrganisationSignupRequestForm, UpdateAdminCourseForm, UpdateAdminUserForm,
    UpdateOrganisationForm,
};
use crate::services::email_verification_service::{
    create_email_verification_token, verification_url,
};
use crate::services::mailer_service::{MailRequest, send_mail_message};
use crate::services::organisation_service::delete_organisation_and_dependents;
// Password Hash Helper
fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);

    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|err| err.to_string())
}

fn singapore_now() -> DateTime<FixedOffset> {
    let singapore_offset =
        FixedOffset::east_opt(8 * 60 * 60).expect("Singapore UTC offset must be valid");

    Utc::now().with_timezone(&singapore_offset)
}

fn required_trimmed(value: &str, field_name: &str) -> Result<String, HttpResponse> {
    let value = value.trim();

    if value.is_empty() {
        Err(HttpResponse::BadRequest().body(format!("{} is required", field_name)))
    } else {
        Ok(value.to_string())
    }
}

fn optional_trimmed_string(
    value: Option<String>,
    field_name: &str,
    max_len: usize,
) -> Result<Option<String>, HttpResponse> {
    let Some(value) = value else {
        return Ok(None);
    };

    let value = value.trim();

    if value.is_empty() {
        return Ok(None);
    }

    if value.len() > max_len {
        return Err(HttpResponse::BadRequest().body(format!(
            "{} must not exceed {} characters",
            field_name, max_len
        )));
    }

    Ok(Some(value.to_string()))
}

fn optional_normalized_slug(value: Option<String>) -> Result<Option<String>, HttpResponse> {
    let Some(value) = optional_trimmed_string(value, "Organisation slug", 255)? else {
        return Ok(None);
    };
    let normalized = value.to_lowercase();
    let has_valid_chars = normalized
        .chars()
        .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-');

    if !has_valid_chars || normalized.starts_with('-') || normalized.ends_with('-') {
        return Err(HttpResponse::BadRequest().body(
            "Organisation slug may only contain lowercase letters, numbers, and hyphens, and cannot start or end with a hyphen",
        ));
    }

    Ok(Some(normalized))
}

async fn email_is_used_by_another_user(
    db: &DatabaseConnection,
    email: &str,
    excluded_user_id: Option<i32>,
) -> Result<bool, HttpResponse> {
    let mut query = users::Entity::find().filter(users::Column::Email.eq(email));

    if let Some(user_id) = excluded_user_id {
        query = query.filter(users::Column::UserId.ne(user_id));
    }

    query
        .one(db)
        .await
        .map(|user| user.is_some())
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!("Email lookup error: {}", err))
        })
}

async fn organisation_name_is_used_by_another_org(
    db: &DatabaseConnection,
    org_name: &str,
    excluded_org_id: Option<i32>,
) -> Result<bool, HttpResponse> {
    let organisations = organisations::Entity::find().all(db).await.map_err(|err| {
        HttpResponse::InternalServerError().body(format!("Organisation lookup error: {}", err))
    })?;

    Ok(organisations.into_iter().any(|organisation| {
        organisation.org_name.eq_ignore_ascii_case(org_name)
            && Some(organisation.org_id) != excluded_org_id
    }))
}

async fn organisation_slug_is_used_by_another_org(
    db: &DatabaseConnection,
    org_slug: &str,
    excluded_org_id: Option<i32>,
) -> Result<bool, HttpResponse> {
    let mut query = organisations::Entity::find().filter(organisations::Column::OrgSlug.eq(org_slug));

    if let Some(org_id) = excluded_org_id {
        query = query.filter(organisations::Column::OrgId.ne(org_id));
    }

    query
        .one(db)
        .await
        .map(|organisation| organisation.is_some())
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!("Organisation slug lookup error: {}", err))
        })
}

async fn ensure_organisation_exists(
    db: &DatabaseConnection,
    org_id: Option<i32>,
) -> Result<(), HttpResponse> {
    let Some(org_id) = org_id else {
        return Ok(());
    };

    if org_id <= 0 {
        return Err(HttpResponse::BadRequest().body("Organisation must be a valid record"));
    }

    match organisations::Entity::find_by_id(org_id).one(db).await {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(HttpResponse::BadRequest().body("Selected organisation does not exist")),
        Err(err) => Err(HttpResponse::InternalServerError()
            .body(format!("Organisation lookup error: {}", err))),
    }
}

async fn ensure_role_exists(db: &DatabaseConnection, role_id: Option<i32>) -> Result<(), HttpResponse> {
    let Some(role_id) = role_id else {
        return Ok(());
    };

    if role_id <= 0 {
        return Err(HttpResponse::BadRequest().body("Role must be a valid record"));
    }

    match roles::Entity::find_by_id(role_id).one(db).await {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(HttpResponse::BadRequest().body("Selected role does not exist")),
        Err(err) => Err(HttpResponse::InternalServerError().body(format!("Role lookup error: {}", err))),
    }
}

async fn ensure_user_exists(db: &DatabaseConnection, user_id: Option<i32>, label: &str) -> Result<(), HttpResponse> {
    let Some(user_id) = user_id else {
        return Ok(());
    };

    if user_id <= 0 {
        return Err(HttpResponse::BadRequest().body(format!("{} must be a valid record", label)));
    }

    match users::Entity::find_by_id(user_id).one(db).await {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(HttpResponse::BadRequest().body(format!("Selected {} does not exist", label.to_lowercase()))),
        Err(err) => Err(HttpResponse::InternalServerError().body(format!("{} lookup error: {}", label, err))),
    }
}

async fn ensure_course_exists(db: &DatabaseConnection, course_id: Option<i32>) -> Result<(), HttpResponse> {
    let Some(course_id) = course_id else {
        return Ok(());
    };

    if course_id <= 0 {
        return Err(HttpResponse::BadRequest().body("Course must be a valid record"));
    }

    match courses::Entity::find_by_id(course_id).one(db).await {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(HttpResponse::BadRequest().body("Selected course does not exist")),
        Err(err) => Err(HttpResponse::InternalServerError().body(format!("Course lookup error: {}", err))),
    }
}

fn validate_optional_http_url(value: Option<&str>, field_name: &str) -> Result<(), HttpResponse> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(());
    };

    if value.len() > 2048 {
        return Err(
            HttpResponse::BadRequest().body(format!("{} must not exceed 2048 characters", field_name))
        );
    }

    if value.starts_with("https://") || value.starts_with("http://") {
        Ok(())
    } else {
        Err(HttpResponse::BadRequest().body(format!(
            "{} must start with http:// or https://",
            field_name
        )))
    }
}

fn validate_optional_course_image_url(value: Option<&str>) -> Result<(), HttpResponse> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(());
    };

    if value.starts_with("/static/images/course-presets/")
        && !value.contains("..")
        && (value.ends_with(".jpg") || value.ends_with(".jpeg") || value.ends_with(".png") || value.ends_with(".webp"))
    {
        return Ok(());
    }

    validate_optional_http_url(Some(value), "Background image URL")
}

fn validate_optional_website_url(website_url: Option<&str>) -> Result<(), HttpResponse> {
    validate_optional_http_url(website_url, "Website URL")
}

#[derive(Serialize)]
pub struct OrganisationSignupRequestDto {
    pub request_id: i32,
    pub org_name: String,
    pub org_slug: String,
    pub org_type: Option<String>,
    pub website_url: Option<String>,
    pub requester_user_id: Option<i32>,
    pub admin_first_name: Option<String>,
    pub admin_last_name: Option<String>,
    pub admin_email: String,
    pub status: String,
    pub approved_by: Option<i32>,
    pub approved_at: Option<DateTime<FixedOffset>>,
    pub rejected_by: Option<i32>,
    pub rejected_at: Option<DateTime<FixedOffset>>,
    pub rejection_reason: Option<String>,
    pub created_at: Option<DateTime<FixedOffset>>,
    pub updated_at: Option<DateTime<FixedOffset>>,
}

impl From<organisation_signup_requests::Model> for OrganisationSignupRequestDto {
    fn from(request: organisation_signup_requests::Model) -> Self {
        Self {
            request_id: request.request_id,
            org_name: request.org_name,
            org_slug: request.org_slug,
            org_type: request.org_type,
            website_url: request.website_url,
            requester_user_id: request.requester_user_id,
            admin_first_name: request.admin_first_name,
            admin_last_name: request.admin_last_name,
            admin_email: request.admin_email,
            status: request.status,
            approved_by: request.approved_by,
            approved_at: request.approved_at,
            rejected_by: request.rejected_by,
            rejected_at: request.rejected_at,
            rejection_reason: request.rejection_reason,
            created_at: request.created_at,
            updated_at: request.updated_at,
        }
    }
}

async fn assign_role_if_missing<C>(
    db: &C,
    user_id: i32,
    role_name: roles::RoleName,
) -> Result<(), sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    let role = roles::Entity::find()
        .filter(roles::Column::RoleName.eq(role_name))
        .one(db)
        .await?
        .ok_or_else(|| sea_orm::DbErr::RecordNotFound("Role not found in database.".to_string()))?;

    let existing = user_roles::Entity::find()
        .filter(user_roles::Column::UserId.eq(user_id))
        .filter(user_roles::Column::RoleId.eq(role.role_id))
        .one(db)
        .await?;

    if existing.is_none() {
        user_roles::ActiveModel {
            user_id: Set(user_id),
            role_id: Set(role.role_id),
        }
        .insert(db)
        .await?;
    }

    Ok(())
}

fn send_organisation_approval_email(
    email: &str,
    org_name: &str,
    verification_token: Option<&str>,
) -> Result<(), String> {
    let mut body = format!(
        "Your SkillUp LMS organisation request for \"{}\" has been approved.\n\nYou can now sign in and manage your organisation workspace.",
        org_name
    );

    if let Some(token) = verification_token {
        body.push_str(&format!(
            "\n\nBefore signing in, verify the organisation admin email address here:\n{}",
            verification_url(token)
        ));
    }

    send_mail_message(MailRequest {
        to: email.to_string(),
        subject: "Your SkillUp LMS organisation has been approved".to_string(),
        body,
        is_html: false,
    })
}

fn send_organisation_rejection_email(
    email: &str,
    org_name: &str,
    reason: Option<&str>,
) -> Result<(), String> {
    let mut body = format!(
        "Your SkillUp LMS organisation request for \"{}\" was not approved at this time.",
        org_name
    );

    if let Some(reason) = reason.filter(|value| !value.trim().is_empty()) {
        body.push_str(&format!("\n\nReason:\n{}", reason.trim()));
    }

    send_mail_message(MailRequest {
        to: email.to_string(),
        subject: "SkillUp LMS organisation request update".to_string(),
        body,
        is_html: false,
    })
}

fn validate_course_payment(
    is_paid: Option<bool>,
    price_cents: Option<i32>,
    currency: Option<&str>,
) -> Result<(), HttpResponse> {
    if price_cents.is_some_and(|price| price < 0) {
        return Err(HttpResponse::BadRequest().body("Course price cannot be negative"));
    }

    if is_paid.unwrap_or(false) {
        if !price_cents.is_some_and(|price| price > 0) {
            return Err(
                HttpResponse::BadRequest().body("Paid courses must have a price greater than zero")
            );
        }

        if !currency.is_some_and(|value| value.eq_ignore_ascii_case("SGD")) {
            return Err(HttpResponse::BadRequest().body("Paid courses currently support SGD only"));
        }
    } else if price_cents.is_some() || currency.is_some() {
        return Err(
            HttpResponse::BadRequest().body("Unpaid courses must not have a price or currency")
        );
    }

    Ok(())
}

fn parse_course_status(status: Option<String>) -> Result<CourseStatus, HttpResponse> {
    match status
        .unwrap_or_else(|| "draft".to_string())
        .to_lowercase()
        .as_str()
    {
        "draft" => Ok(CourseStatus::Draft),
        "published" => Ok(CourseStatus::Published),
        "archived" => Ok(CourseStatus::Archived),
        _ => Err(HttpResponse::BadRequest()
            .body("Invalid course status. Use draft, published, or archived")),
    }
}

fn course_status_label(status: &CourseStatus) -> &'static str {
    match status {
        CourseStatus::Draft => "draft",
        CourseStatus::Published => "published",
        CourseStatus::Archived => "archived",
    }
}

fn cents_to_sgd(cents: i32) -> String {
    format!("{:.2}", cents as f64 / 100.0)
}

fn analytics_month_label(year: i32, month: u32) -> String {
    let month_name = match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "Unknown",
    };

    format!("{} {:02}", month_name, year.rem_euclid(100))
}

// Organisation CRUD
pub async fn get_all_organisations(db: &DatabaseConnection) -> HttpResponse {
    match organisations::Entity::find().all(db).await {
        Ok(orgs) => HttpResponse::Ok().json(orgs),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn get_all_roles(db: &DatabaseConnection) -> HttpResponse {
    match roles::Entity::find().all(db).await {
        Ok(role_list) => HttpResponse::Ok().json(role_list),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn get_admin_analytics_data(
    db: &DatabaseConnection,
    selected_org_id: Option<i32>,
) -> HttpResponse {
    let (organisations_result, courses_result, enrollments_result, payments_result) = tokio::join!(
        organisations::Entity::find().all(db),
        courses::Entity::find().all(db),
        enrollments::Entity::find().all(db),
        payments::Entity::find().all(db),
    );

    let organisations = match organisations_result {
        Ok(items) => items,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Organisation analytics lookup error: {}", err));
        }
    };
    let courses = match courses_result {
        Ok(items) => items,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Course analytics lookup error: {}", err));
        }
    };
    let enrollments = match enrollments_result {
        Ok(items) => items,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Enrollment analytics lookup error: {}", err));
        }
    };
    let payments = match payments_result {
        Ok(items) => items,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Payment analytics lookup error: {}", err));
        }
    };

    let organisation_options = organisations
        .iter()
        .map(|organisation| {
            json!({
                "orgId": organisation.org_id,
                "orgName": organisation.org_name,
            })
        })
        .collect::<Vec<_>>();
    let organisation_names = organisations
        .iter()
        .map(|organisation| (organisation.org_id, organisation.org_name.clone()))
        .collect::<HashMap<_, _>>();
    let selected_org_ids = selected_org_id
        .map(|org_id| HashSet::from([org_id]))
        .unwrap_or_else(|| {
            organisations
                .iter()
                .map(|organisation| organisation.org_id)
                .collect()
        });
    let filtered_courses = courses
        .iter()
        .filter(|course| {
            selected_org_id.is_none()
                || course
                    .org_id
                    .is_some_and(|org_id| selected_org_ids.contains(&org_id))
        })
        .cloned()
        .collect::<Vec<_>>();
    let filtered_course_ids = filtered_courses
        .iter()
        .map(|course| course.course_id)
        .collect::<HashSet<_>>();
    let filtered_enrollments = enrollments
        .iter()
        .filter(|enrollment| filtered_course_ids.contains(&enrollment.course_id))
        .collect::<Vec<_>>();
    let filtered_payments = payments
        .iter()
        .filter(|payment| filtered_course_ids.contains(&payment.course_id))
        .collect::<Vec<_>>();
    let successful_payments = filtered_payments
        .iter()
        .filter(|payment| payment.payment_status.eq_ignore_ascii_case("SUCCEEDED"))
        .collect::<Vec<_>>();
    let enrollments_by_course =
        filtered_enrollments
            .iter()
            .fold(HashMap::new(), |mut map, enrollment| {
                *map.entry(enrollment.course_id).or_insert(0usize) += 1;
                map
            });
    let successful_payments_by_course =
        successful_payments
            .iter()
            .fold(HashMap::new(), |mut map, payment| {
                *map.entry(payment.course_id).or_insert(0i32) += payment.amount_cents;
                map
            });

    let mut course_analytics = filtered_courses
        .iter()
        .map(|course| {
            let enrollment_count = *enrollments_by_course.get(&course.course_id).unwrap_or(&0);
            let is_paid = course.is_paid.unwrap_or(false);
            let price_cents = course.price_cents.unwrap_or(0);
            let confirmed_revenue_cents = *successful_payments_by_course
                .get(&course.course_id)
                .unwrap_or(&0);
            let projected_revenue_cents = if is_paid {
                enrollment_count as i32 * price_cents
            } else {
                0
            };
            let org_name = course
                .org_id
                .and_then(|org_id| organisation_names.get(&org_id).cloned())
                .unwrap_or_else(|| "No organisation".to_string());

            json!({
                "courseId": course.course_id,
                "courseName": course.name.clone().unwrap_or_else(|| format!("Course #{}", course.course_id)),
                "orgId": course.org_id,
                "orgName": org_name,
                "status": course_status_label(&course.status),
                "isPaid": is_paid,
                "currency": course.currency.clone().unwrap_or_else(|| "SGD".to_string()),
                "enrollmentCount": enrollment_count,
                "confirmedRevenueCents": confirmed_revenue_cents,
                "grossProfitCents": confirmed_revenue_cents,
                "projectedRevenueCents": projected_revenue_cents,
            })
        })
        .collect::<Vec<_>>();

    course_analytics.sort_by(|a, b| {
        b["enrollmentCount"]
            .as_u64()
            .cmp(&a["enrollmentCount"].as_u64())
            .then_with(|| {
                b["confirmedRevenueCents"]
                    .as_i64()
                    .cmp(&a["confirmedRevenueCents"].as_i64())
            })
    });

    let mut organisation_analytics = organisations
        .iter()
        .filter(|organisation| {
            selected_org_id.is_none() || Some(organisation.org_id) == selected_org_id
        })
        .map(|organisation| {
            let org_courses = course_analytics
                .iter()
                .filter(|course| course["orgId"].as_i64() == Some(organisation.org_id as i64))
                .collect::<Vec<_>>();
            let course_count = org_courses.len();
            let enrollment_count = org_courses
                .iter()
                .map(|course| course["enrollmentCount"].as_u64().unwrap_or(0) as usize)
                .sum::<usize>();
            let confirmed_revenue_cents = org_courses
                .iter()
                .map(|course| course["confirmedRevenueCents"].as_i64().unwrap_or(0) as i32)
                .sum::<i32>();
            let projected_revenue_cents = org_courses
                .iter()
                .map(|course| course["projectedRevenueCents"].as_i64().unwrap_or(0) as i32)
                .sum::<i32>();

            json!({
                "orgId": organisation.org_id,
                "orgName": organisation.org_name,
                "courseCount": course_count,
                "enrollmentCount": enrollment_count,
                "confirmedRevenueCents": confirmed_revenue_cents,
                "grossProfitCents": confirmed_revenue_cents,
                "projectedRevenueCents": projected_revenue_cents,
            })
        })
        .collect::<Vec<_>>();

    organisation_analytics.sort_by(|a, b| {
        b["confirmedRevenueCents"]
            .as_i64()
            .cmp(&a["confirmedRevenueCents"].as_i64())
            .then_with(|| {
                b["enrollmentCount"]
                    .as_u64()
                    .cmp(&a["enrollmentCount"].as_u64())
            })
    });

    let paid_enrollments = course_analytics
        .iter()
        .filter(|course| course["isPaid"].as_bool().unwrap_or(false))
        .map(|course| course["enrollmentCount"].as_u64().unwrap_or(0) as usize)
        .sum::<usize>();
    let paid_courses = course_analytics
        .iter()
        .filter(|course| course["isPaid"].as_bool().unwrap_or(false))
        .count();
    let free_courses = course_analytics.len().saturating_sub(paid_courses);
    let confirmed_revenue_cents = course_analytics
        .iter()
        .map(|course| course["confirmedRevenueCents"].as_i64().unwrap_or(0) as i32)
        .sum::<i32>();
    let course_statuses = course_analytics
        .iter()
        .fold(BTreeMap::new(), |mut map, course| {
            let status = course["status"].as_str().unwrap_or("unknown").to_string();
            *map.entry(status).or_insert(0usize) += 1;
            map
        })
        .into_iter()
        .map(|(status, count)| json!({ "status": status, "count": count }))
        .collect::<Vec<_>>();
    let revenue_trend = successful_payments
        .iter()
        .fold(BTreeMap::new(), |mut map, payment| {
            let date = payment.paid_at.unwrap_or(payment.created_at);
            *map.entry((date.year(), date.month())).or_insert(0i32) += payment.amount_cents;
            map
        })
        .into_iter()
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|((year, month), revenue_cents)| {
            json!({
                "label": analytics_month_label(year, month),
                "revenueCents": revenue_cents,
            })
        })
        .collect::<Vec<_>>();
    let summary_rows = organisation_analytics
        .iter()
        .map(|item| {
            let confirmed = item["confirmedRevenueCents"].as_i64().unwrap_or(0) as i32;
            let gross = item["grossProfitCents"].as_i64().unwrap_or(0) as i32;
            let projected = item["projectedRevenueCents"].as_i64().unwrap_or(0) as i32;

            json!({
                "organisation": item["orgName"].as_str().unwrap_or(""),
                "courses": item["courseCount"].as_u64().unwrap_or(0),
                "enrollments": item["enrollmentCount"].as_u64().unwrap_or(0),
                "confirmed_revenue_sgd": cents_to_sgd(confirmed),
                "gross_profit_sgd": cents_to_sgd(gross),
                "projected_revenue_sgd": cents_to_sgd(projected),
            })
        })
        .collect::<Vec<_>>();
    let mut detail_rows = course_analytics
        .iter()
        .map(|course| {
            let confirmed = course["confirmedRevenueCents"].as_i64().unwrap_or(0) as i32;
            let projected = course["projectedRevenueCents"].as_i64().unwrap_or(0) as i32;

            json!({
                "record_type": "course",
                "organisation": course["orgName"].as_str().unwrap_or("No organisation"),
                "course": course["courseName"].as_str().unwrap_or("Course"),
                "course_status": course["status"].as_str().unwrap_or(""),
                "is_paid_course": if course["isPaid"].as_bool().unwrap_or(false) { "yes" } else { "no" },
                "enrollments": course["enrollmentCount"].as_u64().unwrap_or(0),
                "confirmed_revenue_sgd": cents_to_sgd(confirmed),
                "projected_revenue_sgd": cents_to_sgd(projected),
                "payment_id": "",
                "payment_status": "",
                "paid_at": "",
            })
        })
        .collect::<Vec<_>>();
    let courses_by_id = course_analytics
        .iter()
        .filter_map(|course| {
            course["courseId"]
                .as_i64()
                .map(|id| (id as i32, course.clone()))
        })
        .collect::<HashMap<_, _>>();

    detail_rows.extend(filtered_payments.iter().map(|payment| {
        let course = courses_by_id.get(&payment.course_id);
        let confirmed = if payment.payment_status.eq_ignore_ascii_case("SUCCEEDED") {
            payment.amount_cents
        } else {
            0
        };

        json!({
            "record_type": "payment",
            "organisation": course.and_then(|course| course["orgName"].as_str()).unwrap_or("No organisation"),
            "course": course.and_then(|course| course["courseName"].as_str()).unwrap_or("Course"),
            "course_status": course.and_then(|course| course["status"].as_str()).unwrap_or(""),
            "is_paid_course": if course.and_then(|course| course["isPaid"].as_bool()).unwrap_or(false) { "yes" } else { "no" },
            "enrollments": enrollments_by_course.get(&payment.course_id).copied().unwrap_or(0),
            "confirmed_revenue_sgd": cents_to_sgd(confirmed),
            "projected_revenue_sgd": "",
            "payment_id": payment.payment_id.to_string(),
            "payment_status": payment.payment_status,
            "paid_at": payment.paid_at.map(|date| date.to_rfc3339()).unwrap_or_else(|| payment.created_at.to_rfc3339()),
        })
    }));

    HttpResponse::Ok().json(json!({
        "selectedOrgId": selected_org_id,
        "organisations": organisation_options,
        "totals": {
            "totalEnrollments": filtered_enrollments.len(),
            "paidEnrollments": paid_enrollments,
            "paidCourses": paid_courses,
            "freeCourses": free_courses,
            "totalCourses": course_analytics.len(),
            "successfulPaymentCount": successful_payments.len(),
            "confirmedRevenueCents": confirmed_revenue_cents,
        },
        "courseAnalytics": course_analytics,
        "organisationAnalytics": organisation_analytics,
        "courseStatuses": course_statuses,
        "revenueTrend": revenue_trend,
        "summaryRows": summary_rows,
        "detailRows": detail_rows,
    }))
}

pub async fn get_organisation_signup_requests(db: &DatabaseConnection) -> HttpResponse {
    match organisation_signup_requests::Entity::find()
        .order_by_desc(organisation_signup_requests::Column::CreatedAt)
        .all(db)
        .await
    {
        Ok(requests) => HttpResponse::Ok().json(
            requests
                .into_iter()
                .map(OrganisationSignupRequestDto::from)
                .collect::<Vec<_>>(),
        ),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn approve_organisation_signup_request(
    db: &DatabaseConnection,
    request_id: i32,
    approved_by: Option<i32>,
) -> HttpResponse {
    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Approval transaction error: {}", err));
        }
    };

    let request = match organisation_signup_requests::Entity::find_by_id(request_id)
        .one(&txn)
        .await
    {
        Ok(Some(request)) => request,
        Ok(None) => return HttpResponse::NotFound().body("Organisation signup request not found"),
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Request lookup error: {}", err));
        }
    };

    if request.status != "pending" {
        return HttpResponse::BadRequest().body("Only pending requests can be approved");
    }

    match organisations::Entity::find()
        .filter(organisations::Column::OrgSlug.eq(request.org_slug.clone()))
        .one(&txn)
        .await
    {
        Ok(Some(_)) => return HttpResponse::BadRequest().body("Organisation slug already exists"),
        Ok(None) => {}
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Organisation slug lookup error: {}", err));
        }
    }

    let now = singapore_now();
    let new_org = organisations::ActiveModel {
        org_name: Set(request.org_name.clone()),
        org_slug: Set(Some(request.org_slug.clone())),
        org_type: Set(request.org_type.clone()),
        website_url: Set(request.website_url.clone()),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    };

    let inserted_org = match new_org.insert(&txn).await {
        Ok(org) => org,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Create organisation error: {}", err));
        }
    };

    let (admin_email, verification_token) = if let Some(user_id) = request.requester_user_id {
        let user = match users::Entity::find_by_id(user_id).one(&txn).await {
            Ok(Some(user)) => user,
            Ok(None) => return HttpResponse::BadRequest().body("Requesting user was not found"),
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("User lookup error: {}", err));
            }
        };

        if user.org_id.is_some() {
            return HttpResponse::BadRequest()
                .body("Requesting user already belongs to an organisation");
        }

        let mut active_user = user.into_active_model();
        active_user.org_id = Set(Some(inserted_org.org_id));
        active_user.updated_at = Set(Some(singapore_now()));
        let updated_user = match active_user.update(&txn).await {
            Ok(user) => user,
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("User update error: {}", err));
            }
        };

        if let Err(err) = assign_role_if_missing(
            &txn,
            updated_user.user_id,
            roles::RoleName::OrganisationAdmin,
        )
        .await
        {
            return HttpResponse::InternalServerError()
                .body(format!("Role assignment error: {}", err));
        }

        (updated_user.email, None)
    } else {
        match users::Entity::find()
            .filter(users::Column::Email.eq(request.admin_email.clone()))
            .one(&txn)
            .await
        {
            Ok(Some(_)) => return HttpResponse::BadRequest().body("Admin email already exists"),
            Ok(None) => {}
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Admin email lookup error: {}", err));
            }
        }

        let Some(password_hash) = request.admin_password_hash.clone() else {
            return HttpResponse::BadRequest().body("Request is missing admin password details");
        };

        let new_user = users::ActiveModel {
            first_name: Set(request.admin_first_name.clone().unwrap_or_default()),
            last_name: Set(request.admin_last_name.clone().unwrap_or_default()),
            email: Set(request.admin_email.clone()),
            password_hash: Set(Some(password_hash)),
            auth_provider: Set("password".to_string()),
            org_id: Set(Some(inserted_org.org_id)),
            email_verified: Set(false),
            must_change_password: Set(false),
            created_at: Set(Some(singapore_now())),
            updated_at: Set(Some(singapore_now())),
            ..Default::default()
        };

        let inserted_user = match new_user.insert(&txn).await {
            Ok(user) => user,
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Admin user insert error: {}", err));
            }
        };

        if let Err(err) = assign_role_if_missing(
            &txn,
            inserted_user.user_id,
            roles::RoleName::OrganisationAdmin,
        )
        .await
        {
            return HttpResponse::InternalServerError()
                .body(format!("Role assignment error: {}", err));
        }

        let token = match create_email_verification_token(&txn, inserted_user.user_id).await {
            Ok(token) => token,
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Verification token error: {}", err));
            }
        };

        (inserted_user.email, Some(token))
    };

    let mut active_request = request.into_active_model();
    active_request.status = Set("approved".to_string());
    active_request.approved_by = Set(approved_by);
    active_request.approved_at = Set(Some(singapore_now()));
    active_request.updated_at = Set(Some(singapore_now()));

    if let Err(err) = active_request.update(&txn).await {
        return HttpResponse::InternalServerError().body(format!("Request update error: {}", err));
    }

    if let Err(err) = txn.commit().await {
        return HttpResponse::InternalServerError().body(format!("Approval commit error: {}", err));
    }

    if let Err(err) = send_organisation_approval_email(
        &admin_email,
        &inserted_org.org_name,
        verification_token.as_deref(),
    ) {
        return HttpResponse::InternalServerError()
            .body(format!("Organisation approved, but email failed: {}", err));
    }

    HttpResponse::Ok().body("Organisation signup request approved")
}

pub async fn reject_organisation_signup_request(
    db: &DatabaseConnection,
    request_id: i32,
    rejected_by: Option<i32>,
    body: RejectOrganisationSignupRequestForm,
) -> HttpResponse {
    let request = match organisation_signup_requests::Entity::find_by_id(request_id)
        .one(db)
        .await
    {
        Ok(Some(request)) => request,
        Ok(None) => return HttpResponse::NotFound().body("Organisation signup request not found"),
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Request lookup error: {}", err));
        }
    };

    if request.status != "pending" {
        return HttpResponse::BadRequest().body("Only pending requests can be rejected");
    }

    let reason = body
        .reason
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let now = singapore_now();
    let mut active_request = request.clone().into_active_model();
    active_request.status = Set("rejected".to_string());
    active_request.rejected_by = Set(rejected_by);
    active_request.rejected_at = Set(Some(now));
    active_request.rejection_reason = Set(reason.clone());
    active_request.updated_at = Set(Some(now));

    match active_request.update(db).await {
        Ok(_) => {
            if let Err(err) = send_organisation_rejection_email(
                &request.admin_email,
                &request.org_name,
                reason.as_deref(),
            ) {
                return HttpResponse::InternalServerError()
                    .body(format!("Request rejected, but email failed: {}", err));
            }

            HttpResponse::Ok().body("Organisation signup request rejected")
        }
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Request update error: {}", err))
        }
    }
}

pub async fn create_organisation_service(
    db: &DatabaseConnection,
    body: CreateOrganisationForm,
) -> HttpResponse {
    if let Err(errors) = body.validate() {
        return HttpResponse::BadRequest().body(format!("Validation error: {}", errors));
    }

    let org_name = match required_trimmed(&body.org_name, "Organisation name") {
        Ok(name) => name,
        Err(response) => return response,
    };

    match organisation_name_is_used_by_another_org(db, &org_name, None).await {
        Ok(true) => return HttpResponse::Conflict().body("An organisation with this name already exists"),
        Ok(false) => {}
        Err(response) => return response,
    }

    let now = singapore_now();
    let new_org = organisations::ActiveModel {
        org_name: Set(org_name),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    };

    match new_org.insert(db).await {
        Ok(org) => HttpResponse::Ok().json(org),
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Create organisation error: {}", err))
        }
    }
}

pub async fn update_organisation_service(
    db: &DatabaseConnection,
    org_id: i32,
    body: UpdateOrganisationForm,
) -> HttpResponse {
    if let Err(errors) = body.validate() {
        return HttpResponse::BadRequest().body(format!("Validation error: {}", errors));
    }

    let org = match organisations::Entity::find_by_id(org_id).one(db).await {
        Ok(Some(org)) => org,
        Ok(None) => return HttpResponse::NotFound().body("Organisation not found"),
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Database error: {}", err));
        }
    };

    let org_name = match required_trimmed(&body.org_name, "Organisation name") {
        Ok(name) => name,
        Err(response) => return response,
    };
    match organisation_name_is_used_by_another_org(db, &org_name, Some(org_id)).await {
        Ok(true) => return HttpResponse::Conflict().body("An organisation with this name already exists"),
        Ok(false) => {}
        Err(response) => return response,
    }

    let org_slug = match optional_normalized_slug(body.org_slug) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if let Some(org_slug) = org_slug.as_deref() {
        match organisation_slug_is_used_by_another_org(db, org_slug, Some(org_id)).await {
            Ok(true) => return HttpResponse::Conflict().body("Organisation slug already exists"),
            Ok(false) => {}
            Err(response) => return response,
        }
    }
    let org_type = match optional_trimmed_string(body.org_type, "Organisation type", 100) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let website_url = match optional_trimmed_string(body.website_url, "Website URL", 2048) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if let Err(response) = validate_optional_website_url(website_url.as_deref()) {
        return response;
    }

    let mut active_org = org.into_active_model();
    active_org.org_name = Set(org_name);
    active_org.org_slug = Set(org_slug);
    active_org.org_type = Set(org_type);
    active_org.website_url = Set(website_url);
    active_org.updated_at = Set(Some(singapore_now()));

    match active_org.update(db).await {
        Ok(updated_org) => HttpResponse::Ok().json(updated_org),
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Update organisation error: {}", err))
        }
    }
}

pub async fn delete_organisation_service(db: &DatabaseConnection, org_id: i32) -> HttpResponse {
    delete_organisation_and_dependents(db, org_id).await
}

// User CRUD
pub async fn get_all_users(db: &DatabaseConnection) -> HttpResponse {
    match users::Entity::find().all(db).await {
        Ok(users_list) => HttpResponse::Ok().json(users_list),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn create_user_service(
    db: &DatabaseConnection,
    body: CreateAdminUserForm,
) -> HttpResponse {
    if let Err(errors) = body.validate() {
        return HttpResponse::BadRequest().body(format!("Validation error: {}", errors));
    }

    let first_name = match required_trimmed(&body.first_name, "First name") {
        Ok(name) => name,
        Err(response) => return response,
    };
    let last_name = match required_trimmed(&body.last_name, "Last name") {
        Ok(name) => name,
        Err(response) => return response,
    };
    let email = body.email.trim().to_lowercase();

    match email_is_used_by_another_user(db, &email, None).await {
        Ok(true) => return HttpResponse::Conflict().body("A user with this email already exists"),
        Ok(false) => {}
        Err(response) => return response,
    }
    if let Err(response) = ensure_organisation_exists(db, body.org_id).await {
        return response;
    }
    if let Err(response) = ensure_role_exists(db, body.role_id).await {
        return response;
    }

    let password_hash = match hash_password(&body.password) {
        Ok(hash) => hash,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Password hash error: {}", err));
        }
    };

    let role_id = body.role_id;
    let now = singapore_now();
    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Create user transaction error: {}", err));
        }
    };

    let new_user = users::ActiveModel {
        first_name: Set(first_name),
        last_name: Set(last_name),
        email: Set(email),
        password_hash: Set(Some(password_hash)),
        org_id: Set(body.org_id),
        email_verified: Set(false),
        must_change_password: Set(false),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    };

    let inserted_user = match new_user.insert(&txn).await {
        Ok(user) => user,
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Create user error: {}", err));
        }
    };

    if let Some(role_id) = role_id {
        let new_user_role = user_roles::ActiveModel {
            user_id: Set(inserted_user.user_id),
            role_id: Set(role_id),
        };

        if let Err(err) = new_user_role.insert(&txn).await {
            return HttpResponse::InternalServerError()
                .body(format!("User created, but failed to assign role: {}", err));
        }
    }

    if let Err(err) = txn.commit().await {
        return HttpResponse::InternalServerError().body(format!("Create user commit error: {}", err));
    }

    HttpResponse::Ok().json(inserted_user)
}

pub async fn update_user_service(
    db: &DatabaseConnection,
    user_id: i32,
    body: UpdateAdminUserForm,
) -> HttpResponse {
    if let Err(errors) = body.validate() {
        return HttpResponse::BadRequest().body(format!("Validation error: {}", errors));
    }

    let first_name = match required_trimmed(&body.first_name, "First name") {
        Ok(name) => name,
        Err(response) => return response,
    };
    let last_name = match required_trimmed(&body.last_name, "Last name") {
        Ok(name) => name,
        Err(response) => return response,
    };
    let email = body.email.trim().to_lowercase();

    match email_is_used_by_another_user(db, &email, Some(user_id)).await {
        Ok(true) => return HttpResponse::Conflict().body("A user with this email already exists"),
        Ok(false) => {}
        Err(response) => return response,
    }
    if let Err(response) = ensure_organisation_exists(db, body.org_id).await {
        return response;
    }

    let user = match users::Entity::find_by_id(user_id).one(db).await {
        Ok(Some(user)) => user,
        Ok(None) => return HttpResponse::NotFound().body("User not found"),
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Database error: {}", err));
        }
    };

    let mut active_user = user.into_active_model();

    active_user.first_name = Set(first_name);
    active_user.last_name = Set(last_name);
    active_user.email = Set(email);
    active_user.org_id = Set(body.org_id);
    active_user.updated_at = Set(Some(singapore_now()));

    match active_user.update(db).await {
        Ok(updated_user) => HttpResponse::Ok().json(updated_user),
        Err(err) => HttpResponse::InternalServerError().body(format!("Update user error: {}", err)),
    }
}

pub async fn delete_user_service(db: &DatabaseConnection, user_id: i32) -> HttpResponse {
    match users::Entity::delete_by_id(user_id).exec(db).await {
        Ok(result) => {
            if result.rows_affected == 0 {
                HttpResponse::NotFound().body("User not found")
            } else {
                HttpResponse::Ok().body("User deleted successfully")
            }
        }
        Err(err) => HttpResponse::InternalServerError().body(format!("Delete user error: {}", err)),
    }
}

// Course CRUD
pub async fn get_all_courses(db: &DatabaseConnection) -> HttpResponse {
    match courses::Entity::find().all(db).await {
        Ok(course_list) => HttpResponse::Ok().json(course_list),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn create_course_service(
    db: &DatabaseConnection,
    body: CreateAdminCourseForm,
) -> HttpResponse {
    if let Err(errors) = body.validate() {
        return HttpResponse::BadRequest().body(format!("Validation error: {}", errors));
    }
    let currency = match optional_trimmed_string(body.currency, "Currency", 3) {
        Ok(value) => value.map(|value| value.to_uppercase()),
        Err(response) => return response,
    };
    if let Err(response) = validate_course_payment(body.is_paid, body.price_cents, currency.as_deref()) {
        return response;
    }
    if let Err(response) = ensure_organisation_exists(db, body.org_id).await {
        return response;
    }
    if let Err(response) = ensure_user_exists(db, body.instructor_id, "Instructor").await {
        return response;
    }

    let status = match parse_course_status(body.status) {
        Ok(status) => status,
        Err(response) => return response,
    };
    let course_name = match required_trimmed(&body.name, "Course name") {
        Ok(name) => name,
        Err(response) => return response,
    };
    let description = match optional_trimmed_string(body.description, "Description", 10_000) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let background_image_url =
        match optional_trimmed_string(body.background_image_url, "Background image URL", 2048) {
            Ok(value) => value,
            Err(response) => return response,
        };
    if let Err(response) = validate_optional_course_image_url(background_image_url.as_deref()) {
        return response;
    }
    let now = singapore_now();

    let new_course = courses::ActiveModel {
        name: Set(Some(course_name)),
        org_id: Set(body.org_id),
        instructor_id: Set(body.instructor_id),
        status: Set(status),
        price_cents: Set(body.price_cents),
        currency: Set(currency),
        is_paid: Set(Some(body.is_paid.unwrap_or(false))),
        description: Set(description),
        background_image_url: Set(background_image_url),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    };

    match new_course.insert(db).await {
        Ok(course) => HttpResponse::Ok().json(course),
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Create course error: {}", err))
        }
    }
}
pub async fn update_course_service(
    db: &DatabaseConnection,
    course_id: i32,
    body: UpdateAdminCourseForm,
) -> HttpResponse {
    if let Err(errors) = body.validate() {
        return HttpResponse::BadRequest().body(format!("Validation error: {}", errors));
    }
    let currency = match optional_trimmed_string(body.currency, "Currency", 3) {
        Ok(value) => value.map(|value| value.to_uppercase()),
        Err(response) => return response,
    };
    if let Err(response) = validate_course_payment(body.is_paid, body.price_cents, currency.as_deref()) {
        return response;
    }
    if let Err(response) = ensure_organisation_exists(db, body.org_id).await {
        return response;
    }
    if let Err(response) = ensure_user_exists(db, body.instructor_id, "Instructor").await {
        return response;
    }

    let course = match courses::Entity::find_by_id(course_id).one(db).await {
        Ok(Some(course)) => course,
        Ok(None) => return HttpResponse::NotFound().body("Course not found"),
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Database error: {}", err));
        }
    };

    let course_name = match required_trimmed(&body.name, "Course name") {
        Ok(name) => name,
        Err(response) => return response,
    };
    let description = match optional_trimmed_string(body.description, "Description", 10_000) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let background_image_url =
        match optional_trimmed_string(body.background_image_url, "Background image URL", 2048) {
            Ok(value) => value,
            Err(response) => return response,
        };
    if let Err(response) = validate_optional_course_image_url(background_image_url.as_deref()) {
        return response;
    }
    let mut active_course = course.into_active_model();

    active_course.name = Set(Some(course_name));
    active_course.org_id = Set(body.org_id);
    active_course.instructor_id = Set(body.instructor_id);

    if body.status.is_some() {
        let status = match parse_course_status(body.status) {
            Ok(status) => status,
            Err(response) => return response,
        };

        active_course.status = Set(status);
    }

    if let Some(is_paid) = body.is_paid {
        active_course.is_paid = Set(Some(is_paid));

        if is_paid {
            active_course.price_cents = Set(body.price_cents);
            active_course.currency = Set(currency);
        } else {
            active_course.price_cents = Set(None);
            active_course.currency = Set(None);
        }
    }

    active_course.description = Set(description);
    active_course.background_image_url = Set(background_image_url);
    active_course.updated_at = Set(Some(singapore_now()));

    match active_course.update(db).await {
        Ok(updated_course) => HttpResponse::Ok().json(updated_course),
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Update course error: {}", err))
        }
    }
}

pub async fn delete_course_service(db: &DatabaseConnection, course_id: i32) -> HttpResponse {
    match courses::Entity::delete_by_id(course_id).exec(db).await {
        Ok(result) => {
            if result.rows_affected == 0 {
                HttpResponse::NotFound().body("Course not found")
            } else {
                HttpResponse::Ok().body("Course deleted successfully")
            }
        }
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Delete course error: {}", err))
        }
    }
}

// Enrollment Admin CRUD
pub async fn get_all_enrollments(db: &DatabaseConnection) -> HttpResponse {
    match enrollments::Entity::find().all(db).await {
        Ok(enrollment_list) => HttpResponse::Ok().json(enrollment_list),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

pub async fn admin_enroll_user_service(
    db: &DatabaseConnection,
    body: AdminEnrollmentForm,
) -> HttpResponse {
    if let Err(response) = ensure_user_exists(db, Some(body.user_id), "User").await {
        return response;
    }
    if let Err(response) = ensure_course_exists(db, Some(body.course_id)).await {
        return response;
    }

    match enrollments::Entity::find_by_id((body.user_id, body.course_id))
        .one(db)
        .await
    {
        Ok(Some(_)) => {
            return HttpResponse::BadRequest().body("User is already enrolled in this course");
        }
        Ok(None) => {}
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Enrollment check error: {}", err));
        }
    }

    let now = singapore_now();
    let new_enrollment = enrollments::ActiveModel {
        user_id: Set(body.user_id),
        course_id: Set(body.course_id),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    };

    match new_enrollment.insert(db).await {
        Ok(enrollment) => HttpResponse::Ok().json(enrollment),
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Create enrollment error: {}", err))
        }
    }
}

pub async fn admin_unenroll_user_service(
    db: &DatabaseConnection,
    user_id: i32,
    course_id: i32,
) -> HttpResponse {
    if user_id <= 0 || course_id <= 0 {
        return HttpResponse::BadRequest().body("User and course must be valid records");
    }

    match enrollments::Entity::delete_by_id((user_id, course_id))
        .exec(db)
        .await
    {
        Ok(result) => {
            if result.rows_affected == 0 {
                HttpResponse::NotFound().body("Enrollment not found")
            } else {
                HttpResponse::Ok().body("User unenrolled successfully")
            }
        }
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Delete enrollment error: {}", err))
        }
    }
}
