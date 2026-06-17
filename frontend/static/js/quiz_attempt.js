const pathParts = window.location.pathname.split("/");
const courseId = pathParts[2];
const quizId = pathParts[4];

let quizPayload = null;
let attempt = null;
let isSubmitting = false;
let timerIntervalId = null;

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
    button.innerHTML = `<i class="bi bi-send" aria-hidden="true"></i><span>${escapeHtml(label)}</span>`;
}

function setSubmitVisible(visible) {
    const button = document.getElementById("submit-quiz-btn");

    if (button) {
        button.hidden = !visible;
    }
}

function setTimerText(message, state = "") {
    const timer = document.getElementById("quiz-timer-text");

    if (!timer) {
        return;
    }

    timer.textContent = message;
    timer.className = state ? `quiz-timer-text ${state}` : "quiz-timer-text";
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
    } else {
        parts.push("No time limit");
    }

    if (quiz.max_attempts) {
        parts.push(`${quiz.max_attempts} attempt${quiz.max_attempts === 1 ? "" : "s"}`);
    }

    return parts.join(" / ");
}

function canAttemptQuiz() {
    return quizPayload?.access?.can_attempt === true;
}

function disableQuestionInputs() {
    document
        .querySelectorAll("#quiz-question-list input, #quiz-question-list textarea")
        .forEach((input) => {
            input.disabled = true;
        });
}

function renderQuiz() {
    const quiz = quizPayload.quiz;
    const questionList = document.getElementById("quiz-question-list");
    const readonly = !canAttemptQuiz();
    const disabled = readonly ? "disabled" : "";

    document.getElementById("quiz-course-link").href = `/course/${courseId}`;
    document.getElementById("quiz-attempt-title").textContent = quiz.title || "Quiz";
    document.getElementById("quiz-attempt-meta").textContent = formatQuizMeta(quiz);

    questionList.innerHTML = quizPayload.questions.map((question, index) => {
        if (question.question_type === "mcq") {
            const options = question.options.map((option) => `
                <label class="quiz-option ${readonly ? "readonly" : ""}">
                    <input
                        type="radio"
                        name="question-${question.question_id}"
                        value="${option.option_id}"
                        data-question-id="${question.question_id}"
                        data-question-type="mcq"
                        ${disabled}
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
                    placeholder="${readonly ? "Preview only" : "Type your answer here"}"
                    ${disabled}
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
    if (!canAttemptQuiz()) {
        document.getElementById("quiz-progress-text").textContent = "Preview only";
        return;
    }

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

function formatRemainingTime(totalSeconds) {
    const seconds = Math.max(0, Math.ceil(totalSeconds));
    const minutesPart = Math.floor(seconds / 60);
    const secondsPart = seconds % 60;

    return `${minutesPart}:${String(secondsPart).padStart(2, "0")}`;
}

function stopTimer() {
    if (timerIntervalId) {
        clearInterval(timerIntervalId);
        timerIntervalId = null;
    }
}

function renderStaticTimer(timer) {
    if (!timer?.time_limit_minutes) {
        setTimerText(timer?.message || "No time limit");
        return;
    }

    setTimerText(timer.message || `${timer.time_limit_minutes} minute time limit`);
}

function startTimer(timer) {
    stopTimer();

    if (!timer?.time_limit_minutes) {
        setTimerText(timer?.message || "No time limit");
        return;
    }

    const startedAt = Date.now();
    const initialRemainingSeconds = Number(timer.remaining_seconds);

    if (Number.isFinite(initialRemainingSeconds)) {
        const tick = () => {
            const elapsedSeconds = (Date.now() - startedAt) / 1000;
            const remainingSeconds = initialRemainingSeconds - elapsedSeconds;

            if (remainingSeconds <= 0) {
                stopTimer();
                setTimerText("Time is up", "expired");
                disableQuestionInputs();
                submitQuiz({ autoSubmit: true });
                return;
            }

            setTimerText(formatRemainingTime(remainingSeconds), remainingSeconds <= 60 ? "warning" : "");
        };

        tick();
        timerIntervalId = setInterval(tick, 1000);
        return;
    }

    if (!timer.expires_at) {
        setTimerText(timer.message || `${timer.time_limit_minutes} minute time limit`);
        return;
    }

    const expiresAt = new Date(timer.expires_at).getTime();

    const tick = () => {
        const remainingSeconds = (expiresAt - Date.now()) / 1000;

        if (remainingSeconds <= 0) {
            stopTimer();
            setTimerText("Time is up", "expired");
            disableQuestionInputs();
            submitQuiz({ autoSubmit: true });
            return;
        }

        setTimerText(formatRemainingTime(remainingSeconds), remainingSeconds <= 60 ? "warning" : "");
    };

    tick();
    timerIntervalId = setInterval(tick, 1000);
}

async function submitQuiz(options = {}) {
    if (isSubmitting || !attempt || !quizPayload || !canAttemptQuiz()) {
        return;
    }

    const autoSubmit = Boolean(options.autoSubmit);
    isSubmitting = true;
    setSubmitState(true, autoSubmit ? "Auto-submitting..." : "Submitting...");
    setStatus(autoSubmit ? "Time is up. Submitting your quiz..." : "Submitting quiz...");

    try {
        const answers = collectAnswers();

        for (const answer of answers) {
            if (answer.question_type === "mcq" && answer.selected_option_id === null) {
                continue;
            }

            if (answer.question_type === "long_answer" && !answer.answer_text) {
                continue;
            }

            if (answer.question_type === "mcq") {
                await requestJson("/api/quiz-answers/mcq", {
                    method: "POST",
                    body: JSON.stringify({
                        attempt_id: attempt.attempt_id,
                        question_id: answer.question_id,
                        selected_option_id: answer.selected_option_id,
                        auto_submit: autoSubmit,
                    }),
                });
            } else {
                await requestJson("/api/quiz-answers/long-answer", {
                    method: "POST",
                    body: JSON.stringify({
                        attempt_id: attempt.attempt_id,
                        question_id: answer.question_id,
                        answer_text: answer.answer_text,
                        auto_submit: autoSubmit,
                    }),
                });
            }
        }

        await requestJson(`/api/quiz-attempts/${attempt.attempt_id}/submit${autoSubmit ? "?auto_submit=true" : ""}`, {
            method: "PUT",
        });

        stopTimer();
        setStatus(autoSubmit ? "Time is up. Quiz submitted automatically." : "Quiz submitted.", "success");
        setSubmitState(true, autoSubmit ? "Auto-submitted" : "Submitted");

        if (window.opener && !window.opener.closed) {
            window.opener.refreshQuizAttemptsAfterSubmit?.(Number(quizId));
        }

        setTimeout(() => {
            window.close();
        }, 600);
    } catch (error) {
        isSubmitting = false;
        setStatus(error.message || "Failed to submit quiz.", "error");
        setSubmitState(false, autoSubmit ? "Submit Quiz" : "Submit Quiz");
    }
}

async function init() {
    try {
        setStatus("Loading quiz...");
        quizPayload = await requestJson(`/api/quiz/${quizId}/attempt-view`);
        renderQuiz();

        if (!canAttemptQuiz()) {
            renderStaticTimer(quizPayload.timer);
            setSubmitVisible(false);
            setStatus(quizPayload.access?.message || "You can view the questions but cannot attempt this quiz.");
            return;
        }

        const attemptResponse = await requestJson("/api/quiz-attempts", {
            method: "POST",
            body: JSON.stringify({
                quiz_id: Number(quizId),
            }),
        });

        attempt = attemptResponse.attempt || attemptResponse;
        startTimer(attemptResponse.timer || quizPayload.timer);
        setStatus("");
        setSubmitVisible(true);
        setSubmitState(false);
    } catch (error) {
        renderStaticTimer(quizPayload?.timer);
        setStatus(error.message || "Unable to load quiz.", "error");
        setSubmitState(true);
    }

    document.getElementById("submit-quiz-btn")?.addEventListener("click", submitQuiz);
}

init();
