const pathParts = window.location.pathname.split("/");
const courseId = pathParts[2];
const quizId = new URLSearchParams(window.location.search).get("quiz_id");
const questionList = document.getElementById("question-list");
const questionTemplate = document.getElementById("question-template");
const optionTemplate = document.getElementById("option-template");

let questionCounter = 0;
let courseModules = [];
let editingQuizId = quizId ? Number(quizId) : null;

function populateQuizStartTimeOptions() {
    const timeSelect = document.getElementById("quiz-start-time-input");

    if (!timeSelect || timeSelect.options.length) {
        return;
    }

    for (let hour = 0; hour < 24; hour += 1) {
        for (const minute of ["00", "30"]) {
            const value = `${String(hour).padStart(2, "0")}:${minute}`;
            timeSelect.innerHTML += `<option value="${value}">${value}</option>`;
        }
    }
}

function renumberQuestions() {
    questionList.querySelectorAll(".question-card").forEach((card, index) => {
        card.querySelector("h3").textContent = `Question ${index + 1}`;
    });
}

function updateSummary() {
    const cards = [...questionList.querySelectorAll(".question-card")];
    const mcqCount = cards.filter((card) => card.dataset.questionType === "mcq").length;
    const shortAnswerCount = cards.filter((card) => card.dataset.questionType === "long_answer").length;

    document.getElementById("quiz-question-count").textContent = String(cards.length);
    document.getElementById("quiz-mcq-count").textContent = String(mcqCount);
    document.getElementById("quiz-short-answer-count").textContent = String(shortAnswerCount);
}

function createOption(card, text = "", checked = false) {
    const optionNode = optionTemplate.content.firstElementChild.cloneNode(true);
    const radio = optionNode.querySelector(".correct-option-input");
    const textInput = optionNode.querySelector(".option-text-input");

    radio.name = `correct-option-${card.dataset.questionId}`;
    radio.checked = checked;
    textInput.value = text;

    optionNode.querySelector(".delete-option-btn").addEventListener("click", () => {
        const options = card.querySelectorAll(".option-row");

        if (options.length <= 2) {
            return;
        }

        const wasChecked = radio.checked;
        optionNode.remove();

        if (wasChecked) {
            const firstRadio = card.querySelector(".correct-option-input");
            if (firstRadio) {
                firstRadio.checked = true;
            }
        }
    });

    card.querySelector(".mcq-options").appendChild(optionNode);
}

function setQuestionType(card, type) {
    card.dataset.questionType = type;
    card.querySelector(".question-type-input").value = type;

    if (type === "mcq" && !card.querySelector(".option-row")) {
        createOption(card, "", true);
        createOption(card);
        createOption(card);
        createOption(card);
    }

    updateSummary();
}

function createQuestion(type = "mcq") {
    questionCounter += 1;

    const card = questionTemplate.content.firstElementChild.cloneNode(true);
    card.dataset.questionId = String(questionCounter);

    card.querySelector(".question-type-input").addEventListener("change", (event) => {
        setQuestionType(card, event.target.value);
    });

    card.querySelector(".delete-question-btn").addEventListener("click", () => {
        if (questionList.querySelectorAll(".question-card").length <= 1) {
            return;
        }

        card.remove();
        renumberQuestions();
        updateSummary();
    });

    card.querySelector(".add-option-btn").addEventListener("click", () => {
        createOption(card);
    });

    questionList.appendChild(card);
    setQuestionType(card, type);

    renumberQuestions();
    updateSummary();

    return card;
}

function goBackToCourse() {
    window.location.href = `/course/${courseId}`;
}

function setSaveStatus(message, type = "") {
    const status = document.getElementById("quiz-save-status");

    if (!status) {
        return;
    }

    status.textContent = message;
    status.className = type ? `quiz-save-status ${type}` : "quiz-save-status";
}

function setSaving(isSaving) {
    const saveButton = document.getElementById("save-quiz-draft-btn");
    const previewButton = document.getElementById("preview-quiz-btn");

    if (saveButton) {
        saveButton.disabled = isSaving;
        saveButton.innerHTML = isSaving
            ? '<i class="bi bi-arrow-repeat" aria-hidden="true"></i><span>Saving...</span>'
            : '<i class="bi bi-save" aria-hidden="true"></i><span>Save Draft</span>';
    }

    if (previewButton) {
        previewButton.disabled = isSaving;
    }
}

function collectDraft() {
    return {
        course_id: Number(courseId),
        title: document.getElementById("quiz-title-input").value.trim(),
        description: document.getElementById("quiz-description-input").value.trim(),
        starts_at: document.getElementById("quiz-start-date-input").value
            ? `${document.getElementById("quiz-start-date-input").value}T${document.getElementById("quiz-start-time-input").value || "00:00"}:00`
            : null,
        max_attempts: document.getElementById("quiz-max-attempts-input").value.trim(),
        time_limit: document.getElementById("quiz-time-limit-input").value.trim(),
        prerequisite_module_ids: getSelectedQuizPrerequisiteIds(),
        questions: [...questionList.querySelectorAll(".question-card")].map((card, index) => ({
            position: index + 1,
            question_type: card.dataset.questionType,
            question_text: card.querySelector(".question-text-input").value.trim(),
            points: card.querySelector(".question-points-input").value.trim(),
            options: card.dataset.questionType === "mcq"
                ? [...card.querySelectorAll(".option-row")].map((option, optionIndex) => ({
                    position: optionIndex + 1,
                    option_text: option.querySelector(".option-text-input").value.trim(),
                    is_correct: option.querySelector(".correct-option-input").checked,
                }))
                : [],
        })),
    };
}

function renderQuizPrerequisiteOptions(selectedIds = []) {
    const prerequisiteInput = document.getElementById("quiz-prerequisites-input");

    if (!prerequisiteInput) {
        return;
    }

    const selected = new Set(selectedIds.map(Number));

    const options = courseModules
        .sort((first, second) => Number(first.position) - Number(second.position))
        .map((module) => {
            const moduleId = Number(module.module_id);
            const checked = selected.has(moduleId) ? "checked" : "";
            const label = `${module.position}. ${module.title || "Untitled module"}`;

            return `
                <label class="prerequisite-option">
                    <input type="checkbox" value="${moduleId}" ${checked}>
                    <span>${escapeHtml(label)}</span>
                </label>
            `;
        })
        .join("");

    prerequisiteInput.innerHTML = options || '<p class="prerequisite-empty">No modules available.</p>';
}

function getSelectedQuizPrerequisiteIds() {
    const prerequisiteInput = document.getElementById("quiz-prerequisites-input");

    if (!prerequisiteInput) {
        return [];
    }

    return [...prerequisiteInput.querySelectorAll('input[type="checkbox"]:checked')]
        .map((input) => Number(input.value));
}

async function loadCourseModules() {
    try {
        const response = await fetch(`/api/modules/${courseId}`);

        if (!response.ok) {
            courseModules = [];
            renderQuizPrerequisiteOptions();
            return;
        }

        courseModules = await response.json();
        renderQuizPrerequisiteOptions();
    } catch (error) {
        courseModules = [];
        renderQuizPrerequisiteOptions();
    }
}

function validateDraft(draft) {
    if (!draft.title) {
        return "Please enter a quiz title.";
    }

    if (draft.max_attempts && (!Number.isInteger(Number(draft.max_attempts)) || Number(draft.max_attempts) < 1)) {
        return "Max attempts must be 1 or higher.";
    }

    if (draft.time_limit && (!Number.isInteger(Number(draft.time_limit)) || Number(draft.time_limit) < 1)) {
        return "Time limit must be 1 minute or higher.";
    }

    if (!draft.questions.length) {
        return "Please add at least one question.";
    }

    for (const question of draft.questions) {
        if (!question.question_text) {
            return `Question ${question.position} needs question text.`;
        }

        if (!question.points || !Number.isInteger(Number(question.points)) || Number(question.points) < 1) {
            return `Question ${question.position} needs points of 1 or higher.`;
        }

        if (question.question_type === "mcq") {
            const filledOptions = question.options.filter((option) => option.option_text);

            if (filledOptions.length < 2) {
                return `Question ${question.position} needs at least two MCQ options.`;
            }

            if (!filledOptions.some((option) => option.is_correct)) {
                return `Question ${question.position} needs one correct option selected.`;
            }
        }
    }

    return "";
}

function escapeHtml(value) {
    return String(value)
        .replaceAll("&", "&amp;")
        .replaceAll("<", "&lt;")
        .replaceAll(">", "&gt;")
        .replaceAll('"', "&quot;")
        .replaceAll("'", "&#039;");
}

function renderPreview() {
    const draft = collectDraft();
    const validationMessage = validateDraft(draft);

    if (validationMessage) {
        setSaveStatus(validationMessage, "error");
        return;
    }

    const previewPanel = document.getElementById("quiz-preview-panel");
    const previewContent = document.getElementById("quiz-preview-content");

    previewContent.innerHTML = draft.questions.map((question) => {
        const questionType = question.question_type === "mcq" ? "MCQ" : "Short answer";
        const options = question.question_type === "mcq"
            ? `<ul>${question.options
                .filter((option) => option.option_text)
                .map((option) => `
                    <li class="${option.is_correct ? "preview-correct" : ""}">
                        ${escapeHtml(option.option_text)}${option.is_correct ? " (correct)" : ""}
                    </li>
                `)
                .join("")}</ul>`
            : "<p>Short answer response field</p>";

        return `
            <article class="preview-question">
                <h3>Question ${question.position}: ${escapeHtml(question.question_text)}</h3>
                <p>${questionType} · ${escapeHtml(question.points)} point${Number(question.points) === 1 ? "" : "s"}</p>
                ${options}
            </article>
        `;
    }).join("");

    previewPanel.hidden = false;
    previewPanel.scrollIntoView({ behavior: "smooth", block: "start" });
    setSaveStatus("Preview updated.", "success");
}

async function postJson(url, payload) {
    const response = await fetch(url, {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
        },
        body: JSON.stringify(payload),
    });

    if (!response.ok) {
        const message = await response.text();
        throw new Error(message || "Request failed.");
    }

    return response.json();
}

async function putJson(url, payload) {
    const response = await fetch(url, {
        method: "PUT",
        headers: {
            "Content-Type": "application/json",
        },
        body: JSON.stringify(payload),
    });

    if (!response.ok) {
        const message = await response.text();
        throw new Error(message || "Request failed.");
    }

    return response.text();
}

async function getJson(url) {
    const response = await fetch(url);

    if (!response.ok) {
        const message = await response.text();
        throw new Error(message || "Request failed.");
    }

    return response.json();
}

function setDateTimeInputs(value) {
    if (!value) {
        document.getElementById("quiz-start-date-input").value = "";
        document.getElementById("quiz-start-time-input").value = "00:00";
        return;
    }

    const normalized = String(value).replace(" ", "T");
    const [date, time = "00:00"] = normalized.split("T");

    document.getElementById("quiz-start-date-input").value = date || "";
    document.getElementById("quiz-start-time-input").value = time.slice(0, 5) || "00:00";
}

function populateQuestion(question, options = []) {
    const card = createQuestion(question.question_type || "mcq");

    card.querySelector(".question-text-input").value = question.question_text || "";
    card.querySelector(".question-points-input").value = question.points || 1;

    if (question.question_type === "mcq") {
        const optionsWrap = card.querySelector(".mcq-options");
        optionsWrap.innerHTML = "";

        const sortedOptions = [...options].sort((first, second) => Number(first.position) - Number(second.position));
        sortedOptions.forEach((option) => {
            createOption(card, option.option_text || "", Boolean(option.is_correct));
        });
    }

    updateSummary();
}

async function loadExistingQuiz() {
    if (!editingQuizId) {
        return false;
    }

    setSaveStatus("Loading quiz...");

    const quiz = await getJson(`/api/quiz/${editingQuizId}/draft`);

    document.querySelector(".quiz-builder-header h1").textContent = "Edit Quiz";
    document.querySelector("#save-quiz-draft-btn span").textContent = "Save Changes";
    document.getElementById("quiz-title-input").value = quiz.title || "";
    document.getElementById("quiz-description-input").value = quiz.description || "";
    document.getElementById("quiz-max-attempts-input").value = quiz.max_attempts ?? "";
    document.getElementById("quiz-time-limit-input").value = quiz.time_limit ?? "";
    setDateTimeInputs(quiz.starts_at);
    renderQuizPrerequisiteOptions(quiz.prerequisite_module_ids || []);

    const questions = Array.isArray(quiz.questions) ? quiz.questions : [];
    questionList.innerHTML = "";
    questionCounter = 0;

    for (const question of [...questions].sort((first, second) => Number(first.position) - Number(second.position))) {
        const options = question.question_type === "mcq" && Array.isArray(question.options)
            ? question.options
            : [];
        populateQuestion(question, Array.isArray(options) ? options : []);
    }

    if (!questionList.querySelector(".question-card")) {
        createQuestion("mcq");
    }

    setSaveStatus("");
    return true;
}

async function saveDraft() {
    const draft = collectDraft();
    const validationMessage = validateDraft(draft);

    if (validationMessage) {
        setSaveStatus(validationMessage, "error");
        return;
    }

    try {
        setSaving(true);
        setSaveStatus("Saving quiz draft...");

        const quizPayload = {
            course_id: draft.course_id,
            title: draft.title,
            description: draft.description || null,
            max_attempts: draft.max_attempts ? Number(draft.max_attempts) : null,
            time_limit: draft.time_limit ? Number(draft.time_limit) : null,
            starts_at: draft.starts_at,
            prerequisite_module_ids: draft.prerequisite_module_ids,
            questions: draft.questions.map((question) => ({
                question_type: question.question_type,
                question_text: question.question_text,
                position: Number(question.position),
                points: Number(question.points),
                options: question.question_type === "mcq"
                    ? question.options
                        .filter((option) => option.option_text)
                        .map((option) => ({
                            option_text: option.option_text,
                            is_correct: option.is_correct,
                            position: Number(option.position),
                        }))
                    : [],
            })),
        };

        if (editingQuizId) {
            await putJson(`/api/quiz/${editingQuizId}/draft`, quizPayload);
        } else {
            await postJson("/api/quiz/draft", quizPayload);
        }

        setSaveStatus(editingQuizId ? "Quiz updated." : "Quiz draft saved.", "success");
        window.setTimeout(goBackToCourse, 700);
    } catch (error) {
        setSaveStatus(error.message || "Failed to save quiz draft.", "error");
        setSaving(false);
    }
}

async function init() {
    document.getElementById("quiz-back-link").href = `/course/${courseId}`;
    populateQuizStartTimeOptions();
    await loadCourseModules();

    try {
        const loadedExisting = await loadExistingQuiz();
        if (!loadedExisting) {
            createQuestion("mcq");
        }
    } catch (error) {
        setSaveStatus(error.message || "Failed to load quiz.", "error");
        createQuestion("mcq");
    }

    document.getElementById("add-question-btn").addEventListener("click", () => {
        createQuestion("mcq");
    });

    document.getElementById("save-quiz-draft-btn").addEventListener("click", saveDraft);
    document.getElementById("preview-quiz-btn").addEventListener("click", renderPreview);
    document.getElementById("close-preview-btn").addEventListener("click", () => {
        document.getElementById("quiz-preview-panel").hidden = true;
    });
}

init();
