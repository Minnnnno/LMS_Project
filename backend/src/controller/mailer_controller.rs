use actix_web::{web, Responder};

use crate::services::mailer_service::{self, MailRequest};

pub async fn send_mail(mail: web::Json<MailRequest>) -> impl Responder {
    mailer_service::send_mail(mail.into_inner()).await
}
