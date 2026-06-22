const pathParts = window.location.pathname.split("/");
const courseId = pathParts[2];
const quizId = pathParts[4];

let quizPayload = null;
let attempt = null;
let isSubmitting = false;
let timerIntervalId = null;
let autosaveTimeoutId = null;
let autosaveQueue = Promise.resolve();

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

function renderQuestionNavigation() {
    const navList = document.getElementById("quiz-question-nav-list");

    if (!navList) {
        return;
    }

    navList.innerHTML = quizPayload.questions.map((question, index) => `
        <button
            class="quiz-question-nav-btn"
            type="button"
            data-question-id="${question.question_id}"
            aria-label="Go to question ${index + 1}"
        >
            ${index + 1}
        </button>
    `).join("");
}

function updateQuestionNavigation() {
    const answers = collectAnswers();
    const answeredQuestionIds = new Set(
        answers
            .filter((answer) => {
                if (answer.question_type === "mcq") {
                    return answer.selected_option_id !== null;
                }

                return Boolean(answer.answer_text);
            })
            .map((answer) => answer.question_id)
    );

    document.querySelectorAll(".quiz-question-nav-btn").forEach((button) => {
        const questionId = Number(button.dataset.questionId);
        button.classList.toggle("answered", answeredQuestionIds.has(questionId));
    });
}

function setActiveQuestionNavigation(questionId) {
    document.querySelectorAll(".quiz-question-nav-btn").forEach((button) => {
        button.classList.toggle("active", Number(button.dataset.questionId) === questionId);
    });
}

function setupQuestionNavigation() {
    const navList = document.getElementById("quiz-question-nav-list");

    if (!navList) {
        return;
    }

    navList.addEventListener("click", (event) => {
        const button = event.target.closest(".quiz-question-nav-btn");

        if (!button) {
            return;
        }

        const questionId = Number(button.dataset.questionId);
        const card = document.querySelector(`.quiz-question-card[data-question-id="${questionId}"]`);

        if (!card) {
            return;
        }

        card.scrollIntoView({ behavior: "smooth", block: "start" });
        setActiveQuestionNavigation(questionId);
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

    renderQuestionNavigation();
    if (quizPayload.questions.length > 0) {
        setActiveQuestionNavigation(quizPayload.questions[0].question_id);
    }
    questionList.addEventListener("input", updateProgress);
    questionList.addEventListener("change", updateProgress);
    setupQuestionNavigation();
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

function setAutosaveStatus(message, type = "") {
    const status = document.getElementById("quiz-autosave-status");
    if (status) {
        status.textContent = message;
        status.className = type ? `quiz-autosave-status ${type}` : "quiz-autosave-status";
    }
}

function restoreAnswers(savedAnswers) {
    for (const answer of Array.isArray(savedAnswers) ? savedAnswers : []) {
        if (answer.selected_option_id !== null) {
            const option = document.querySelector(
                `input[name="question-${answer.question_id}"][value="${answer.selected_option_id}"]`
            );
            if (option) {
                option.checked = true;
            }
        }
        if (answer.answer_text !== null) {
            const input = document.querySelector(`textarea[data-question-id="${answer.question_id}"]`);
            if (input) {
                input.value = answer.answer_text;
            }
        }
    }
    updateProgress();
}

function answerPayload() {
    return collectAnswers().map((answer) => ({
        question_id: answer.question_id,
        selected_option_id: answer.selected_option_id ?? null,
        answer_text: answer.answer_text ?? null,
    }));
}

function saveAnswers(force = false) {
    if (!attempt || (isSubmitting && !force) || !canAttemptQuiz()) {
        return Promise.resolve();
    }

    setAutosaveStatus("Saving...");
    const request = autosaveQueue
        .catch(() => undefined)
        .then(() => requestJson(`/api/quiz-attempts/${attempt.attempt_id}/answers`, {
            method: "PUT",
            body: JSON.stringify({ answers: answerPayload() }),
        }));
    autosaveQueue = request;

    return request.then(() => {
        setAutosaveStatus("Saved", "success");
    }).catch((error) => {
        setAutosaveStatus("Save failed", "error");
        throw error;
    });
}

function scheduleAutosave(event) {
    if (isSubmitting) {
        return;
    }
    clearTimeout(autosaveTimeoutId);
    if (event.target.matches('input[data-question-type="mcq"]')) {
        saveAnswers().catch(() => undefined);
        return;
    }
    autosaveTimeoutId = setTimeout(() => saveAnswers().catch(() => undefined), 800);
}

function flushAnswers() {
    clearTimeout(autosaveTimeoutId);
    return saveAnswers(true);
}

function updateProgress() {
    if (!canAttemptQuiz()) {
        document.getElementById("quiz-progress-text").textContent = "Preview only";
        updateQuestionNavigation();
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
    updateQuestionNavigation();
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
        try {
            await flushAnswers();
        } catch (error) {
            if (!autoSubmit) {
                throw error;
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
        quizPayload = await requestJson(`/api/quiz/${quizId}/attempt`, { method: "POST" });
        renderQuiz();

        if (!canAttemptQuiz()) {
            renderStaticTimer(quizPayload.timer);
            setSubmitVisible(false);
            setStatus(quizPayload.access?.message || "You can view the questions but cannot attempt this quiz.");
            return;
        }

        attempt = quizPayload.attempt;
        restoreAnswers(quizPayload.answers);
        document.getElementById("quiz-question-list")?.addEventListener("input", scheduleAutosave);
        startTimer(quizPayload.timer);
        setStatus("");
        setAutosaveStatus(quizPayload.answers.length ? "Saved answers restored" : "Autosave ready", "success");
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
