const pathParts = window.location.pathname.split("/");
const courseId = pathParts[2];
const quizId = pathParts[4];

let quizPayload = null;
let attempt = null;
let isSubmitting = false;

function setStatus(message, type = "") {
    const status = document.getElementById("quiz-attempt-status");

    if (!status) {
        return;
    }

    status.textContent = message;
    status.className = type ? `quiz-attempt-status ${type}` : "quiz-attempt-status";
}

function setSubmitState(disabled, label = "Submit Quiz") {
    const button = document.getElementById("submit-quiz-btn");

    if (!button) {
        return;
    }

    button.disabled = disabled;
    button.innerHTML = `<i class="bi bi-send" aria-hidden="true"></i><span>${label}</span>`;
}

function escapeHtml(value) {
    return String(value ?? "")
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#39;");
}

async function requestJson(url, options = {}) {
    const response = await fetch(url, {
        headers: {
            "Content-Type": "application/json",
            ...(options.headers || {}),
        },
        ...options,
    });

    if (!response.ok) {
        const message = await response.text();
        throw new Error(message || "Request failed.");
    }

    const contentType = response.headers.get("content-type") || "";

    if (contentType.includes("application/json")) {
        return response.json();
    }

    return response.text();
}

function formatQuizMeta(quiz) {
    const parts = [];

    if (quiz.time_limit) {
        parts.push(`${quiz.time_limit} min`);
    }

    if (quiz.max_attempts) {
        parts.push(`${quiz.max_attempts} attempt${quiz.max_attempts === 1 ? "" : "s"}`);
    }

    return parts.join(" / ");
}

function renderQuiz() {
    const quiz = quizPayload.quiz;
    const questionList = document.getElementById("quiz-question-list");

    document.getElementById("quiz-course-link").href = `/course/${courseId}`;
    document.getElementById("quiz-attempt-title").textContent = quiz.title || "Quiz";
    document.getElementById("quiz-attempt-meta").textContent = formatQuizMeta(quiz);

    questionList.innerHTML = quizPayload.questions.map((question, index) => {
        if (question.question_type === "mcq") {
            const options = question.options.map((option) => `
                <label class="quiz-option">
                    <input
                        type="radio"
                        name="question-${question.question_id}"
                        value="${option.option_id}"
                        data-question-id="${question.question_id}"
                        data-question-type="mcq"
                    >
                    <span>${escapeHtml(option.option_text)}</span>
                </label>
            `).join("");

            return `
                <article class="quiz-question-card" data-question-id="${question.question_id}" data-question-type="mcq">
                    <h2>Question ${index + 1}: ${escapeHtml(question.question_text)}</h2>
                    <p class="quiz-question-points">${question.points} point${question.points === 1 ? "" : "s"}</p>
                    <div class="quiz-option-list">${options}</div>
                </article>
            `;
        }

        return `
            <article class="quiz-question-card" data-question-id="${question.question_id}" data-question-type="long_answer">
                <h2>Question ${index + 1}: ${escapeHtml(question.question_text)}</h2>
                <p class="quiz-question-points">${question.points} point${question.points === 1 ? "" : "s"}</p>
                <textarea
                    class="quiz-long-answer"
                    data-question-id="${question.question_id}"
                    data-question-type="long_answer"
                    placeholder="Type your answer here"
                ></textarea>
            </article>
        `;
    }).join("");

    questionList.addEventListener("input", updateProgress);
    questionList.addEventListener("change", updateProgress);
    updateProgress();
}

function collectAnswers() {
    return quizPayload.questions.map((question) => {
        if (question.question_type === "mcq") {
            const selected = document.querySelector(`input[name="question-${question.question_id}"]:checked`);
            return {
                question_id: question.question_id,
                question_type: "mcq",
                selected_option_id: selected ? Number(selected.value) : null,
            };
        }

        const answerInput = document.querySelector(`textarea[data-question-id="${question.question_id}"]`);
        return {
            question_id: question.question_id,
            question_type: "long_answer",
            answer_text: answerInput?.value.trim() || "",
        };
    });
}

function updateProgress() {
    const answers = collectAnswers();
    const answered = answers.filter((answer) => {
        if (answer.question_type === "mcq") {
            return answer.selected_option_id !== null;
        }

        return Boolean(answer.answer_text);
    }).length;

    document.getElementById("quiz-progress-text").textContent =
        `${answered} of ${answers.length} answered`;
}

async function submitQuiz() {
    if (isSubmitting || !attempt || !quizPayload) {
        return;
    }

    isSubmitting = true;
    setSubmitState(true, "Submitting...");
    setStatus("Submitting quiz...");

    try {
        const answers = collectAnswers();

        for (const answer of answers) {
            if (answer.question_type === "mcq") {
                await requestJson("/api/quiz-answers/mcq", {
                    method: "POST",
                    body: JSON.stringify({
                        attempt_id: attempt.attempt_id,
                        question_id: answer.question_id,
                        selected_option_id: answer.selected_option_id,
                    }),
                });
            } else {
                await requestJson("/api/quiz-answers/long-answer", {
                    method: "POST",
                    body: JSON.stringify({
                        attempt_id: attempt.attempt_id,
                        question_id: answer.question_id,
                        answer_text: answer.answer_text,
                    }),
                });
            }
        }

        await requestJson(`/api/quiz-attempts/${attempt.attempt_id}/submit`, {
            method: "PUT",
        });

        setStatus("Quiz submitted.", "success");
        setSubmitState(true, "Submitted");

        if (window.opener && !window.opener.closed) {
            window.opener.refreshQuizAttemptsAfterSubmit?.(Number(quizId));
        }

        setTimeout(() => {
            window.close();
        }, 600);
    } catch (error) {
        isSubmitting = false;
        setStatus(error.message || "Failed to submit quiz.", "error");
        setSubmitState(false);
    }
}

async function init() {
    try {
        setStatus("Loading quiz...");
        quizPayload = await requestJson(`/api/quiz/${quizId}/attempt-view`);
        attempt = await requestJson("/api/quiz-attempts", {
            method: "POST",
            body: JSON.stringify({
                quiz_id: Number(quizId),
            }),
        });
        renderQuiz();
        setStatus("");
        setSubmitState(false);
    } catch (error) {
        setStatus(error.message || "Unable to load quiz.", "error");
        setSubmitState(true);
    }

    document.getElementById("submit-quiz-btn")?.addEventListener("click", submitQuiz);
}

init();
