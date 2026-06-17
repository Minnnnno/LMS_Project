// Quiz list, attempt review, and quiz attempt modal behavior for course details.
async function loadQuizzes() {
    try {
        const response = await axios.get("/api/quiz/" + courseId);
        const quizzes = Array.isArray(response.data) ? response.data : [];
        currentQuizzes = quizzes;
        await loadQuizAttemptStatuses();
        const quizList = document.getElementById("quiz-list");

        if (!quizList) {
            return;
        }

        quizList.innerHTML = "";

        if (!quizzes.length) {
            quizList.innerHTML = isInstructor
                ? '<p class="quiz-empty">No quizzes yet. Use Add Quiz to create one.</p>'
                : '<p class="quiz-empty">No quizzes available.</p>';
            return;
        }

        quizzes.forEach((quiz) => {
            const status = quizAttemptStatuses[quiz.quiz_id] || null;
            const studentStatus = !isInstructor && status
                ? `<div class="quiz-attempt-state ${status.can_attempt ? "" : "blocked"}">${escapeHtml(formatQuizAttemptStatus(status))}</div>`
                : "";
            const rowClass = !isInstructor && status && !status.can_attempt
                ? "quiz-row quiz-row-disabled"
                : "quiz-row";
            const adminButtons = isInstructor
                ? `
                    <div class="module-actions">
                        <button class="module-action-btn" onclick="openQuizAttempts(event, ${quiz.quiz_id})">Attempts</button>
                        <button class="module-action-btn edit-btn" onclick="editQuiz(event, ${quiz.quiz_id})">Edit</button>
                        <button class="module-action-btn delete-btn" onclick="deleteQuiz(event, ${quiz.quiz_id})">Delete</button>
                    </div>
                `
                : "";

            quizList.innerHTML += `
                <div class="${rowClass}" onclick="openQuizAttempt(${quiz.quiz_id})">
                    <div>
                        <div class="quiz-title">${escapeHtml(quiz.title || "Untitled quiz")}</div>
                        <div class="quiz-subtitle">${escapeHtml(formatQuizMeta(quiz))}</div>
                        ${studentStatus}
                    </div>
                    ${adminButtons}
                </div>
            `;
        });
    } catch (error) {
        currentQuizzes = [];
        const quizList = document.getElementById("quiz-list");
        if (quizList) {
            quizList.innerHTML = '<p class="quiz-empty">Unable to load quizzes right now.</p>';
        }
        console.error("Failed to load quizzes:", error);
    }
}

async function loadQuizAttemptStatuses() {
    quizAttemptStatuses = {};

    if (isInstructor || !isEnrolled) {
        return;
    }

    try {
        const response = await axios.get(`/api/quiz-attempts/my/course/${courseId}/status`);
        const statuses = Array.isArray(response.data) ? response.data : [];
        quizAttemptStatuses = statuses.reduce((map, status) => {
            map[status.quiz_id] = status;
            return map;
        }, {});
    } catch (error) {
        if (error.response?.status !== 401) {
            console.error("Failed to load quiz attempt statuses:", error);
        }
    }
}

function formatQuizDate(value) {
    if (!value) {
        return "Available anytime";
    }

    const date = parseApiDateTime(value);

    if (Number.isNaN(date.getTime())) {
        return value;
    }

    return date.toLocaleString("en-SG", {
        dateStyle: "medium",
        timeStyle: "short",
        timeZone: SG_TIME_ZONE,
    });
}

function formatQuizMeta(quiz) {
    const parts = [formatQuizDate(quiz.starts_at)];

    if (quiz.time_limit) {
        parts.push(`${quiz.time_limit} min`);
    }

    if (quiz.max_attempts) {
        parts.push(`${quiz.max_attempts} attempt${quiz.max_attempts === 1 ? "" : "s"}`);
    }

    return parts.join(" - ");
}

function formatQuizAttemptStatus(status) {
    if (!status) {
        return "";
    }

    if (!status.can_attempt) {
        return status.has_submitted_attempt ? "Already attempted" : status.message;
    }

    if (status.has_submitted_attempt) {
        return status.message ? `Already attempted / ${status.message}` : "Already attempted";
    }

    return status.message;
}

function editQuiz(event, quizId) {
    event.stopPropagation();
    window.location.href = `/course/${courseId}/quiz-builder?quiz_id=${quizId}`;
}

function renderStudentQuizReviewAnswer(answer) {
    if (answer.question_type === "long_answer") {
        return `
            <div class="quiz-review-answer">
                <h4>${escapeHtml(answer.question_text)}</h4>
                <p class="grade-meta">Short answer - ${escapeHtml(answer.points)} point${answer.points === 1 ? "" : "s"}</p>
                <p>${escapeHtml(answer.answer_text || "No answer submitted.")}</p>
                <div class="staff-grade-readonly">
                    <span>Score: ${escapeHtml(answer.score === null || answer.score === undefined ? "Not marked" : formatGradeNumber(answer.score))} / ${escapeHtml(answer.points)}</span>
                    ${answer.feedback ? `<p class="dropbox-history-note"><strong>Feedback:</strong> ${escapeHtml(answer.feedback)}</p>` : ""}
                </div>
            </div>
        `;
    }

    const isCorrect = answer.score !== null && Number(answer.score) === Number(answer.points);

    return `
        <div class="quiz-review-answer">
            <h4>${escapeHtml(answer.question_text)}</h4>
            <p class="grade-meta">MCQ</p>
            <div class="staff-grade-readonly">
                <span>Your answer: ${escapeHtml(answer.selected_option_text || "No option selected")}</span>
                <span>Correct answer: ${escapeHtml(answer.correct_option_text || "No correct option set")}</span>
                <span>Score: ${escapeHtml(answer.score === null || answer.score === undefined ? "0" : formatGradeNumber(answer.score))} / ${escapeHtml(answer.points)}</span>
                <span>${isCorrect ? "Correct" : "Incorrect"}</span>
            </div>
        </div>
    `;
}

function renderStudentQuizReview(review) {
    const answers = Array.isArray(review.answers) ? review.answers : [];

    return `
        <div class="staff-submission-list">
            <div class="staff-submission-item">
                <div class="staff-submission-head">
                    <div>
                        <strong>Score: ${escapeHtml(formatGradeNumber(review.total_score ?? 0))} / ${escapeHtml(formatGradeNumber(review.max_score))}</strong>
                        <span>${escapeHtml(formatAssignmentDate(review.submitted_at))}</span>
                    </div>
                </div>
                ${answers.map(renderStudentQuizReviewAnswer).join("")}
            </div>
        </div>
    `;
}

async function openMyQuizAttemptReview(attemptId, title = "Quiz") {
    const modal = document.getElementById("quiz-attempts-modal");
    const list = document.getElementById("quiz-attempts-list");
    const heading = document.getElementById("quiz-attempts-title");

    if (heading) {
        heading.textContent = `${title} Answers`;
    }

    if (list) {
        list.innerHTML = '<p class="grades-empty">Loading answers...</p>';
    }

    if (modal) {
        modal.style.display = "flex";
    }

    try {
        const response = await axios.get(`/api/quiz-attempts/my/${attemptId}/review`);
        if (list) {
            list.innerHTML = renderStudentQuizReview(response.data);
        }
    } catch (error) {
        if (list) {
            list.innerHTML = `<p class="grades-error">${escapeHtml(error.response?.data || "Unable to load quiz answers.")}</p>`;
        }
    }
}

function getQuizAttemptScoreLabel(attempt) {
    if (attempt.total_score === null || attempt.total_score === undefined) {
        return `Pending / ${attempt.max_score}`;
    }

    return `${formatGradeNumber(attempt.total_score)} / ${formatGradeNumber(attempt.max_score)}`;
}

function renderQuizAttemptAnswer(answer, savedAnswerId = null) {
    const score = answer.score ?? "";
    const feedback = answer.feedback || "";
    const answerType = answer.question_type;
    const saveStatus = Number(savedAnswerId) === Number(answer.answer_id)
        ? '<p class="quiz-answer-save-status success" role="status">Changes are saved.</p>'
        : "";

    if (answerType === "long_answer") {
        const gradeControls = answer.answer_id
            ? `
                <div class="staff-grade-form">
                    <label for="quiz-answer-score-${answer.answer_id}">Score out of ${escapeHtml(answer.points)}</label>
                    <input id="quiz-answer-score-${answer.answer_id}" type="number" min="0" max="${answer.points}" step="1" value="${escapeHtml(score)}">
                    <label for="quiz-answer-feedback-${answer.answer_id}">Feedback</label>
                    <textarea id="quiz-answer-feedback-${answer.answer_id}" rows="2">${escapeHtml(feedback)}</textarea>
                    <div class="staff-grade-actions">
                        <button type="button" onclick="saveQuizAnswerGrade(${answer.answer_id}, ${answer.points})">Save Mark</button>
                    </div>
                    <p id="quiz-answer-save-status-${answer.answer_id}" class="quiz-answer-save-status" role="status" aria-live="polite"></p>
                    ${saveStatus}
                </div>
            `
            : '<div class="staff-grade-readonly"><span>No answer submitted.</span></div>';

        return `
            <div class="quiz-review-answer">
                <h4>${escapeHtml(answer.question_text)}</h4>
                <p class="grade-meta">Short answer - ${escapeHtml(answer.points)} point${answer.points === 1 ? "" : "s"}</p>
                <p>${escapeHtml(answer.answer_text || "No answer submitted.")}</p>
                ${gradeControls}
            </div>
        `;
    }

    const isCorrect = answer.score !== null && Number(answer.score) === Number(answer.points);
    return `
        <div class="quiz-review-answer">
            <h4>${escapeHtml(answer.question_text)}</h4>
            <p class="grade-meta">MCQ - auto marked</p>
            <div class="staff-grade-readonly">
                <span>Selected: ${escapeHtml(answer.selected_option_text || "No option selected")}</span>
                <span>Correct: ${escapeHtml(answer.correct_option_text || "No correct option set")}</span>
                <span>Score: ${escapeHtml(score === "" ? "0" : formatGradeNumber(score))} / ${escapeHtml(answer.points)}</span>
                <span>${isCorrect ? "Correct" : "Incorrect"}</span>
            </div>
        </div>
    `;
}

function renderQuizAttempts(attempts) {
    const attemptRows = Array.isArray(attempts) ? attempts : [];

    if (!attemptRows.length) {
        return '<p class="grades-empty">No student attempts yet.</p>';
    }

    return `
        <div class="staff-submission-list">
            ${attemptRows.map((attempt) => {
                return `
                <button type="button" class="staff-submission-item quiz-attempt-summary" onclick="showQuizAttemptDetail(${attempt.attempt_id})">
                    <div class="staff-submission-head">
                        <div>
                            <strong>${escapeHtml(attempt.student_name || "Student")}</strong>
                            <span>Student ID: ${escapeHtml(attempt.user_id)}</span>
                            <span>${escapeHtml(attempt.student_email || "")}</span>
                        </div>
                        <div class="staff-submission-meta">
                            <span>${attempt.submitted_at ? "Submitted" : "In progress"}</span>
                            <span>${attempt.is_graded ? "Graded" : "Not graded"}</span>
                            <span>${escapeHtml(formatAssignmentDate(attempt.submitted_at || attempt.started_at))}</span>
                            <span>${escapeHtml(getQuizAttemptScoreLabel(attempt))}</span>
                        </div>
                    </div>
                </button>
            `;
            }).join("")}
        </div>
    `;
}

function renderQuizAttemptDetail(attempt, savedAnswerId = null) {
    if (!attempt) {
        return '<p class="grades-error">Attempt not found.</p>';
    }

    const answers = Array.isArray(attempt.answers) ? attempt.answers : [];

    return `
        <div class="staff-submission-list">
            <button type="button" class="module-action-btn" onclick="showQuizAttemptList()">Back to Attempts</button>
            <div class="staff-submission-item" data-attempt-id="${escapeHtml(attempt.attempt_id)}">
                <div class="staff-submission-head">
                    <div>
                        <strong>${escapeHtml(attempt.student_name || "Student")}</strong>
                        <span>Student ID: ${escapeHtml(attempt.user_id)}</span>
                        <span>${escapeHtml(attempt.student_email || "")}</span>
                    </div>
                    <div class="staff-submission-meta">
                        <span>${attempt.submitted_at ? "Submitted" : "In progress"}</span>
                        <span>${attempt.is_graded ? "Graded" : "Not graded"}</span>
                        <span>${escapeHtml(formatAssignmentDate(attempt.submitted_at || attempt.started_at))}</span>
                        <span>${escapeHtml(getQuizAttemptScoreLabel(attempt))}</span>
                    </div>
                </div>
                ${answers.length ? answers.map((answer) => renderQuizAttemptAnswer(answer, savedAnswerId)).join("") : '<p class="grades-empty">No answers recorded for this attempt.</p>'}
            </div>
        </div>
    `;
}

function showQuizAttemptList() {
    const list = document.getElementById("quiz-attempts-list");
    if (list) {
        list.innerHTML = renderQuizAttempts(currentQuizAttemptRows);
    }
}

function showQuizAttemptDetail(attemptId, savedAnswerId = null) {
    const attempt = currentQuizAttemptRows.find((item) => Number(item.attempt_id) === Number(attemptId));
    const list = document.getElementById("quiz-attempts-list");
    if (list) {
        list.innerHTML = renderQuizAttemptDetail(attempt, savedAnswerId);
    }
}

async function openQuizAttempts(event, quizId) {
    event.stopPropagation();
    currentQuizAttemptsQuizId = quizId;
    currentQuizAttemptRows = [];

    const quiz = currentQuizzes.find((item) => Number(item.quiz_id) === Number(quizId));
    const modal = document.getElementById("quiz-attempts-modal");
    const list = document.getElementById("quiz-attempts-list");
    const title = document.getElementById("quiz-attempts-title");

    if (title) {
        title.textContent = `${quiz?.title || "Quiz"} Attempts`;
    }

    if (list) {
        list.innerHTML = '<p class="grades-empty">Loading student attempts...</p>';
    }

    if (modal) {
        modal.style.display = "flex";
    }

    try {
        const response = await axios.get(`/api/quiz-attempts/quiz/${quizId}`);
        const attempts = Array.isArray(response.data) ? response.data : [];
        currentQuizAttemptRows = attempts;
        if (list) {
            list.innerHTML = renderQuizAttempts(attempts);
        }
    } catch (error) {
        console.error("Failed to load quiz attempts:", error);
        if (list) {
            list.innerHTML = `<p class="grades-error">${escapeHtml(error.response?.data || error.message || "Failed to load quiz attempts.")}</p>`;
        }
    }
}

function closeQuizAttempts() {
    currentQuizAttemptsQuizId = null;
    currentQuizAttemptRows = [];
    const modal = document.getElementById("quiz-attempts-modal");
    if (modal) {
        modal.style.display = "none";
    }
}

async function saveQuizAnswerGrade(answerId, maxPoints) {
    const scoreInput = document.getElementById(`quiz-answer-score-${answerId}`);
    const feedbackInput = document.getElementById(`quiz-answer-feedback-${answerId}`);
    const status = document.getElementById(`quiz-answer-save-status-${answerId}`);
    const score = Number(scoreInput?.value);
    const attemptId = Number(scoreInput?.closest("[data-attempt-id]")?.dataset.attemptId);

    if (!Number.isFinite(score) || score < 0) {
        if (status) {
            status.textContent = "Enter a score of 0 or higher.";
            status.className = "quiz-answer-save-status error";
        }
        showActionMessage("Enter a score of 0 or higher.", "error");
        return;
    }

    if (score > Number(maxPoints)) {
        if (status) {
            status.textContent = `Marks awarded exceed how much this question is worth (${maxPoints}).`;
            status.className = "quiz-answer-save-status error";
        }
        showActionMessage(`Marks awarded exceed how much this question is worth (${maxPoints}).`, "error");
        return;
    }

    try {
        if (status) {
            status.textContent = "Saving changes...";
            status.className = "quiz-answer-save-status";
        }

        await axios.put(`/api/quiz-answers/${answerId}/grade`, {
            score,
            feedback: feedbackInput?.value.trim() || "",
        });

        gradesLoaded = false;
        showActionMessage("Quiz answer marked.", "success");

        if (currentQuizAttemptsQuizId && Number.isFinite(attemptId)) {
            const response = await axios.get(`/api/quiz-attempts/quiz/${currentQuizAttemptsQuizId}`);
            currentQuizAttemptRows = Array.isArray(response.data) ? response.data : [];
            showQuizAttemptDetail(attemptId, answerId);
        }
    } catch (error) {
        if (status) {
            status.textContent = error.response?.data || "Failed to save quiz mark.";
            status.className = "quiz-answer-save-status error";
        }
        showActionMessage(error.response?.data || "Failed to save quiz mark.", "error");
    }
}

function openQuizAttempt(quizId) {
    if (!isInstructor) {
        const status = quizAttemptStatuses[quizId];

        if (status && !status.can_attempt) {
            showActionMessage(status.message || "You cannot attempt this quiz.", "error");
            return;
        }
    }

    window.open(
        `/course/${courseId}/quiz/${quizId}/attempt`,
        `quiz_attempt_${quizId}`,
        "popup=yes,width=980,height=760,resizable=yes,scrollbars=yes"
    );
}

async function refreshQuizAttemptsAfterSubmit() {
    await loadQuizzes();
    showActionMessage("Quiz submitted.", "success");
}

async function deleteQuiz(event, quizId) {
    event.stopPropagation();

    if (!confirm("Delete this quiz?")) {
        return;
    }

    try {
        await axios.delete(`/api/quiz/${quizId}`);
        await loadQuizzes();
        showActionMessage("Quiz deleted.", "success");
    } catch (error) {
        const message = error.response?.data || "Failed to delete quiz.";
        showActionMessage(message, "error");
    }
}
