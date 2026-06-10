use actix_web::HttpResponse;
use lettre::{
    message::Mailbox,
    transport::smtp::authentication::Credentials,
    Message, SmtpTransport, Transport,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct MailRequest {
    pub to: String,
    pub subject: String,
    pub body: String,
}

pub async fn send_mail(mail: MailRequest) -> HttpResponse {
    let smtp_username = std::env::var("SMTP_USERNAME").expect("SMTP_USERNAME not found");
    let smtp_password = std::env::var("SMTP_PASSWORD").expect("SMTP_PASSWORD not found");
    let smtp_host = std::env::var("SMTP_HOST").expect("SMTP_HOST not found");

    let email = Message::builder()
        .from(smtp_username.parse::<Mailbox>().unwrap())
        .to(mail.to.parse::<Mailbox>().unwrap())
        .subject(&mail.subject)
        .body(mail.body)
        .unwrap();

    let creds = Credentials::new(smtp_username, smtp_password);
    let mailer = SmtpTransport::starttls_relay(&smtp_host)
        .unwrap()
        .credentials(creds)
        .build();

    match mailer.send(&email) {
        Ok(_) => HttpResponse::Ok().body("Email sent successfully"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Failed to send email: {}", err)),
    }
}
