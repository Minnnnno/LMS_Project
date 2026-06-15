const pathParts = window.location.pathname.split("/");
const courseId = pathParts[2];
const SG_TIME_ZONE = "Asia/Singapore";
const TIMEZONE_OFFSET_PATTERN = /(Z|[+-]\d{2}:?\d{2})$/i;
let currentCourse = null;
let actionMessageTimer = null;
let isInstructor = false;
let currentEditingModuleId = null;
let currentModules = [];
let currentEditingAssignmentId = null;
let currentAssignments = [];
let currentAssignmentBriefUrl = null;
let gradesLoaded = false;
let isEnrolled = false;
let currentAssignmentDetailsId = null;
let currentQuizzes = [];
let moduleProgressById = new Map();
let quizAttemptStatuses = {};

function goToModuleContent(moduleId) {
    window.location.href = "/module-content/" + moduleId;
}

function getCoursePriceCents(course) {
    if (Number.isFinite(Number(course.price))) {
        const decimalPriceCents = Math.round(Number(course.price) * 100);

        if (decimalPriceCents > 0) {
            return decimalPriceCents;
        }
    }

    if (Number.isFinite(Number(course.price_cents))) {
        return Number(course.price_cents);
    }

    return null;
}

function isPaidCourse(course) {
    const priceCents = getCoursePriceCents(course);
    return Boolean(course.is_paid) || (priceCents !== null && priceCents > 0);
}

function formatCoursePrice(course) {
    if (!isPaidCourse(course)) {
        return "Free course";
    }

    const priceCents = getCoursePriceCents(course);
    const currency = course.currency || "SGD";

    if (priceCents === null) {
        return "Price unavailable";
    }

    return new Intl.NumberFormat("en-SG", {
        style: "currency",
        currency,
    }).format(priceCents / 100);
}

function showActionMessage(message, type = "info") {
    const messageElement = document.getElementById("course-action-message");

    if (!messageElement) {
        return;
    }

    if (actionMessageTimer) {
        clearTimeout(actionMessageTimer);
    }

    messageElement.textContent = message;
    messageElement.className = message
        ? `course-action-message ${type} visible`
        : "course-action-message";

    if (message) {
        actionMessageTimer = setTimeout(() => {
            messageElement.classList.remove("visible");
        }, 4500);
    }
}

function setActionButton(content, disabled = false) {
    const button = document.getElementById("course-action-button");

    if (!button) {
        return;
    }

    button.disabled = disabled;
    button.innerHTML = content;
}

function resetCourseActionButton() {
    if (!currentCourse) {
        return;
    }

    if (isEnrolled) {
        setActionButton('<i class="bi bi-check2" aria-hidden="true"></i><span>Enrolled</span>', true);
        return;
    }

    if (isPaidCourse(currentCourse)) {
        setActionButton('<i class="bi bi-credit-card" aria-hidden="true"></i><span>Buy Course</span>');
    } else {
        setActionButton('<i class="bi bi-check2-circle" aria-hidden="true"></i><span>Enroll Now</span>');
    }
}

function configureCourseAction(course) {
    const price = document.getElementById("course-price");
    const params = new URLSearchParams(window.location.search);

    if (price) {
        price.textContent = formatCoursePrice(course);
    }

    resetCourseActionButton();

    if (params.get("payment") === "cancelled") {
        showActionMessage("Payment was cancelled. You can try again whenever you are ready.", "warning");
    }
}

function refreshCourseDisplay() {
    if (!currentCourse) {
        return;
    }

    document.getElementById("course-title").textContent = currentCourse.name || "Untitled course";

    if (currentCourse.background_image_url) {
        document.getElementById("course-hero").style.backgroundImage =
            `url('${currentCourse.background_image_url}')`;
    }

    configureCourseAction(currentCourse);
}

async function handleCourseAction() {
    if (!currentCourse) {
        return;
    }

    if (isEnrolled) {
        resetCourseActionButton();
        return;
    }

    setActionButton(
        isPaidCourse(currentCourse)
            ? '<i class="bi bi-arrow-repeat" aria-hidden="true"></i><span>Opening checkout...</span>'
            : '<i class="bi bi-arrow-repeat" aria-hidden="true"></i><span>Enrolling...</span>',
        true
    );
    showActionMessage("");

    try {
        if (isPaidCourse(currentCourse)) {
            const response = await axios.post(`/api/courses/${courseId}/checkout`);
            window.location.href = response.data.checkout_url;
            return;
        }

        await axios.post(`/api/courses/${courseId}/enroll`);
        isEnrolled = true;
        resetCourseActionButton();
        await loadCourseProgress();
        await loadCourseModuleProgresses();
        await loadModules();
        showActionMessage("You are enrolled in this course.", "success");
    } catch (error) {
        if (error.response?.status === 401) {
            window.location.href = "/login";
            return;
        }

        const message = error.response?.data || "Something went wrong. Please try again.";
        showActionMessage(message, "error");
        resetCourseActionButton();
    }
}

async function loadModules() {
    try {
        const response = await axios.get("/api/modules/" + courseId);
        const modules = response.data.sort((first, second) => first.position - second.position);
        currentModules = modules;
        const moduleList = document.getElementById("module-list");

        moduleList.innerHTML = "";

        if (modules.length === 0) {
            moduleList.innerHTML = "<p>No modules available.</p>";
            return;
        }

        modules.forEach((module) => {
            const progress = moduleProgressById.get(Number(module.module_id)) || {
                opened: false,
                progress_percent: 0,
            };
            const percent = Math.max(0, Math.min(100, Number(progress.progress_percent || 0)));
            const instructorButtons = isInstructor
                ? `
                    <div class="module-actions">
                        <button class="module-action-btn edit-btn" onclick="editModule(event, ${module.module_id})">Edit</button>
                        <button class="module-action-btn delete-btn" onclick="deleteModule(event, ${module.module_id})">Delete</button>
                    </div>
                `
                : "";
            const progressRing = !isInstructor
                ? `
                    <div class="module-progress-ring" style="--module-progress: ${percent};" aria-label="${percent}% complete">
                        <span>${percent}%</span>
                    </div>
                `
                : "";

            moduleList.innerHTML += `
                <div class="module-row ${percent === 100 ? "completed" : ""}" onclick="goToModuleContent(${module.module_id})">
                    <div class="module-info">
                        <div class="module-title">${escapeHtml(module.title || "Untitled module")}</div>
                    </div>
                    ${instructorButtons}
                    ${progressRing}
                    <span class="module-arrow">&rsaquo;</span>
                </div>
            `;
        });
    } catch (error) {
        console.error("Failed to load modules:", error);
    }
}

async function loadAssignments() {
    try {
        const response = await axios.get("/api/assignment/" + courseId);
        const assignments = response.data;
        currentAssignments = assignments;
        const assignmentList = document.getElementById("assignment-list");
        renderDropboxAssignments();
        if (document.querySelector('.course-tab.active')?.dataset.courseTab === "submissions") {
            renderCourseSubmissionsTab();
        }

        assignmentList.innerHTML = "";

        if (!assignments.length) {
            assignmentList.innerHTML = isInstructor
                ? '<p class="assignment-empty">No assignments yet. Use Add Assignment to create one.</p>'
                : '<p class="assignment-empty">No tasks due.</p>';
            return;
        }

        assignments.forEach((assignment) => {
            const adminButtons = isInstructor
                ? `
                    <div class="module-actions">
                        <button class="module-action-btn edit-btn" onclick="editAssignment(event, ${assignment.assignment_id})">Edit</button>
                        <button class="module-action-btn delete-btn" onclick="deleteAssignment(event, ${assignment.assignment_id})">Delete</button>
                    </div>
                `
                : "";

            assignmentList.innerHTML += `
                <div class="assignment-row" onclick="openAssignmentDetails(${assignment.assignment_id})">
                    <div>
                        <div class="assignment-title">${assignment.title}</div>
                        <div class="assignment-subtitle">Due: ${formatAssignmentDate(assignment.due_date)}</div>
                    </div>
                    ${adminButtons}
                </div>
            `;
        });
    } catch (error) {
        currentAssignments = [];
        renderDropboxAssignments();
        const assignmentList = document.getElementById("assignment-list");
        assignmentList.innerHTML = "<p>No tasks due.</p>";
        console.error("Failed to load assignments:", error);
    }
}

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

    if (isInstructor) {
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

async function loadEnrollmentStatus() {
    try {
        const response = await axios.get(`/api/courses/${courseId}/enrollment-status`);
        isEnrolled = Boolean(response.data.enrolled);
        resetCourseActionButton();
    } catch (error) {
        if (error.response?.status !== 401) {
            console.error("Failed to load enrollment status:", error);
        }
    }
}

async function loadCourseModuleProgresses() {
    moduleProgressById = new Map();

    if (!isEnrolled || isInstructor) {
        return;
    }

    try {
        const response = await axios.get(`/api/courses/${courseId}/module-progress`);
        const progressRows = Array.isArray(response.data) ? response.data : [];

        moduleProgressById = new Map(
            progressRows.map((progress) => [Number(progress.module_id), progress])
        );
    } catch (error) {
        moduleProgressById = new Map();

        if (![401, 403, 404].includes(error.response?.status)) {
            console.error("Failed to load module progress:", error);
        }
    }
}

function hideCourseProgress() {
    const progressCard = document.getElementById("course-progress-card");

    if (progressCard) {
        progressCard.hidden = true;
    }
}

function renderCourseProgress(progress) {
    const progressCard = document.getElementById("course-progress-card");
    const progressSummary = document.getElementById("course-progress-summary");
    const progressPercent = document.getElementById("course-progress-percent");
    const progressFill = document.getElementById("course-progress-fill");

    if (!progressCard || !progressSummary || !progressPercent || !progressFill) {
        return;
    }

    const completedModules = Number(progress.completed_modules || 0);
    const totalModules = Number(progress.total_modules || 0);
    const percent = Math.max(0, Math.min(100, Number(progress.progress_percent || 0)));
    const moduleLabel = totalModules === 1 ? "module" : "modules";

    progressSummary.textContent = totalModules
        ? `${completedModules} of ${totalModules} ${moduleLabel} completed`
        : "No modules available yet";
    progressPercent.textContent = `${percent}%`;
    progressFill.style.width = `${percent}%`;
    progressCard.hidden = false;
}

async function loadCourseProgress() {
    if (!isEnrolled || isInstructor) {
        hideCourseProgress();
        return;
    }

    try {
        const response = await axios.get(`/api/courses/${courseId}/progress`);
        renderCourseProgress(response.data || {});
    } catch (error) {
        hideCourseProgress();

        if (![401, 403, 404].includes(error.response?.status)) {
            console.error("Failed to load course progress:", error);
        }
    }
}

function setActiveCourseTab(tabName) {
    document.querySelectorAll(".course-tab").forEach((tab) => {
        tab.classList.toggle("active", tab.dataset.courseTab === tabName);
    });

    document.getElementById("course-content-panel")
        ?.classList.toggle("active", tabName === "content");
    document.getElementById("course-grades-panel")
        ?.classList.toggle("active", tabName === "grades");
    document.getElementById("course-dropbox-panel")
        ?.classList.toggle("active", tabName === "dropbox");
    document.getElementById("course-submissions-panel")
        ?.classList.toggle("active", tabName === "submissions");

    if (tabName === "grades" && !gradesLoaded) {
        loadGrades();
    }

    if (tabName === "dropbox") {
        renderDropboxAssignments();
    }

    if (tabName === "submissions") {
        renderCourseSubmissionsTab();
    }
}

function setGradeTabsVisible(visible) {
    const tabs = document.getElementById("course-tabs");
    const gradesTab = document.querySelector('.course-tab[data-course-tab="grades"]');
    const dropboxTab = document.querySelector('.course-tab[data-course-tab="dropbox"]');
    const submissionsTab = document.getElementById("course-submissions-tab-btn");

    if (tabs) {
        tabs.style.display = "flex";
    }

    if (gradesTab) {
        gradesTab.style.display = visible ? "inline-flex" : "none";
    }

    if (dropboxTab) {
        dropboxTab.style.display = visible ? "inline-flex" : "none";
    }

    if (submissionsTab) {
        submissionsTab.style.display = isInstructor ? "inline-flex" : "none";
    }

    const activeTab = document.querySelector(".course-tab.active")?.dataset.courseTab;
    if ((!visible && ["grades", "dropbox"].includes(activeTab)) || (visible && activeTab === "submissions")) {
        setActiveCourseTab("content");
    }
}

function formatGradeNumber(value) {
    if (value === null || value === undefined || value === "") {
        return null;
    }

    const numeric = Number(value);

    if (!Number.isFinite(numeric)) {
        return String(value);
    }

    return Number.isInteger(numeric) ? String(numeric) : numeric.toFixed(2);
}

function formatGradeScore(score, maxScore) {
    const formattedScore = formatGradeNumber(score);
    const formattedMaxScore = formatGradeNumber(maxScore);

    if (formattedScore === null) {
        return "Pending";
    }

    if (formattedMaxScore !== null && Number(maxScore) > 0) {
        return `${formattedScore} / ${formattedMaxScore}`;
    }

    return formattedScore;
}

function getGradePercent(score, maxScore) {
    const numericScore = Number(score);
    const numericMax = Number(maxScore);

    if (!Number.isFinite(numericScore) || !Number.isFinite(numericMax) || numericMax <= 0) {
        return null;
    }

    return Math.round((numericScore / numericMax) * 100);
}

function escapeHtml(value) {
    return String(value ?? "")
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#39;");
}

function getGradeDateLabel(value, prefix) {
    if (!value) {
        return "";
    }

    return `${prefix}: ${formatAssignmentDate(value)}`;
}

function buildGradeRow({ title, meta, score, maxScore, feedback }) {
    const hasScore = score !== null && score !== undefined;
    const percent = getGradePercent(score, maxScore);
    const percentLabel = percent === null ? "" : ` (${percent}%)`;

    return `
        <div class="grade-row">
            <div>
                <div class="grade-title">${escapeHtml(title)}</div>
                <div class="grade-meta">${escapeHtml(meta || "No activity yet")}</div>
            </div>
            <div class="grade-score ${hasScore ? "" : "pending"}">
                ${formatGradeScore(score, maxScore)}${percentLabel}
            </div>
            ${feedback ? `<p class="grade-feedback"><strong>Feedback:</strong> ${escapeHtml(feedback)}</p>` : ""}
        </div>
    `;
}

function renderGradeSection(title, rows, emptyMessage) {
    if (!rows.length) {
        return `
            <section class="grade-section">
                <h3>${title}</h3>
                <p class="grades-empty">${emptyMessage}</p>
            </section>
        `;
    }

    return `
        <section class="grade-section">
            <h3>${title}</h3>
            ${rows.join("")}
        </section>
    `;
}

function renderGrades(data) {
    const gradeList = document.getElementById("grades-list");
    const summary = document.getElementById("grades-summary");

    if (!gradeList) {
        return;
    }

    const assignments = data.assignments || [];
    const quizzes = data.quizzes || [];
    const quizMessage = data.quiz_message || null;
    const gradedItems = [
        ...assignments.filter((item) => item.score !== null && item.score !== undefined),
        ...quizzes.filter((item) => item.total_score !== null && item.total_score !== undefined),
    ];

    if (summary) {
        summary.textContent = gradedItems.length
            ? `${gradedItems.length} graded item${gradedItems.length === 1 ? "" : "s"} available.`
            : "No graded items available yet.";
    }

    const assignmentRows = assignments.map((assignment) => {
        const metaParts = [
            getGradeDateLabel(assignment.submitted_at, "Submitted"),
            !assignment.submitted_at ? getGradeDateLabel(assignment.due_date, "Due") : "",
        ].filter(Boolean);

        return buildGradeRow({
            title: assignment.title || "Untitled assignment",
            meta: metaParts.join(" - "),
            score: assignment.score,
            maxScore: assignment.max_score,
            feedback: assignment.feedback,
        });
    });

    const quizRows = quizzes.map((quiz) => buildGradeRow({
        title: quiz.title || "Untitled quiz",
        meta: getGradeDateLabel(quiz.submitted_at, "Submitted") || (quiz.attempt_id ? "Attempt in progress" : ""),
        score: quiz.total_score,
        maxScore: quiz.max_score,
        feedback: null,
    }));

    gradeList.innerHTML =
        renderGradeSection("Assignments", assignmentRows, "No assignments are available for this course.") +
        renderGradeSection("Quizzes", quizRows, quizMessage || "No quizzes are available for this course.");
}

function renderDropboxAssignments() {
    const list = document.getElementById("dropbox-assignment-list");

    if (!list) {
        return;
    }

    const assignments = currentAssignments.filter((assignment) => assignment.allow_file_submission ?? true);

    if (!assignments.length) {
        list.innerHTML = '<p class="grades-empty">No assignment dropboxes are available for this course.</p>';
        return;
    }

    list.innerHTML = assignments.map((assignment) => `
        <div class="grade-row dropbox-row" onclick="openAssignmentDropbox(${assignment.assignment_id})">
            <div>
                <div class="grade-title">${escapeHtml(assignment.title || "Untitled assignment")}</div>
                <div class="grade-meta">Due: ${escapeHtml(formatAssignmentDate(assignment.due_date))}</div>
            </div>
            <div class="grade-score">Open</div>
        </div>
    `).join("");
}

async function loadGrades() {
    const gradeList = document.getElementById("grades-list");
    const summary = document.getElementById("grades-summary");

    if (gradeList) {
        gradeList.innerHTML = '<p class="grades-empty">Loading grades...</p>';
    }

    if (summary) {
        summary.textContent = "";
    }

    try {
        const response = await axios.get(`/api/courses/${courseId}/grades`);
        renderGrades(response.data);
        gradesLoaded = true;
    } catch (error) {
        if (error.response?.status === 401) {
            window.location.href = "/login";
            return;
        }

        const message = error.response?.data || "Failed to load grades.";
        if (gradeList) {
            gradeList.innerHTML = `<p class="grades-error">${escapeHtml(message)}</p>`;
        }
    }
}

function setAssignmentModalTab(tabName) {
    document.querySelectorAll(".assignment-modal-tab").forEach((tab) => {
        tab.classList.toggle("active", tab.dataset.assignmentTab === tabName);
    });

    document.getElementById("assignment-details-tab-panel")
        ?.classList.toggle("active", tabName === "details");
    document.getElementById("assignment-dropbox-tab-panel")
        ?.classList.toggle("active", tabName === "dropbox");
    document.getElementById("assignment-submissions-tab-panel")
        ?.classList.toggle("active", tabName === "submissions");

    if (tabName === "dropbox" && currentAssignmentDetailsId) {
        loadAssignmentSubmissions(currentAssignmentDetailsId);
    }

    if (tabName === "submissions" && currentAssignmentDetailsId) {
        loadStaffAssignmentSubmissions(currentAssignmentDetailsId);
    }
}

function renderCourseSubmissionsTab() {
    const list = document.getElementById("course-submissions-list");

    if (!list) {
        return;
    }

    if (!isInstructor) {
        list.innerHTML = '<p class="grades-empty">Student submissions are available to course staff.</p>';
        return;
    }

    if (!currentAssignments.length) {
        list.innerHTML = '<p class="grades-empty">No assignments have been created yet.</p>';
        return;
    }

    list.innerHTML = currentAssignments.map((assignment) => `
        <div class="grade-row submission-shortcut-row" onclick="openAssignmentDetails(${assignment.assignment_id}, 'submissions')">
            <div>
                <div class="grade-title">${escapeHtml(assignment.title || "Untitled assignment")}</div>
                <div class="grade-meta">Due: ${escapeHtml(formatAssignmentDate(assignment.due_date))}</div>
            </div>
            <div class="grade-score">View Submissions</div>
        </div>
    `).join("");
}

function openAssignmentDropbox(assignmentId) {
    openAssignmentDetails(assignmentId, "dropbox");
}

function openAssignmentDetails(assignmentId, initialTab = "details") {
    const assignment = currentAssignments.find((item) => item.assignment_id === assignmentId);

    if (!assignment) {
        return;
    }

    currentAssignmentDetailsId = assignmentId;
    const submissionsTab = document.getElementById("assignment-submissions-tab-btn");
    if (submissionsTab) {
        submissionsTab.style.display = isInstructor ? "inline-flex" : "none";
    }
    document.getElementById("assignment-details-title").textContent = assignment.title || "Assignment Details";
    document.getElementById("assignment-details-description").textContent =
        assignment.description || "No description provided.";
    document.getElementById("assignment-details-due").textContent = formatAssignmentDate(assignment.due_date);
    document.getElementById("assignment-details-score").textContent = assignment.max_score ?? "Not set";
    document.getElementById("assignment-details-file-type").textContent =
        getFileTypeLabel(assignment.expected_file_type);
    document.getElementById("assignment-details-submission").textContent =
        getAssignmentSubmissionLabel(assignment);

    const briefWrap = document.getElementById("assignment-details-brief-wrap");
    const briefLink = document.getElementById("assignment-details-brief-link");
    if (assignment.assignment_brief_url) {
        briefLink.href = assignment.assignment_brief_url;
        briefWrap.style.display = "block";
    } else {
        briefLink.href = "#";
        briefWrap.style.display = "none";
    }

    const instructionsWrap = document.getElementById("assignment-details-instructions-wrap");
    const instructions = document.getElementById("assignment-details-instructions");
    if (assignment.submission_instructions) {
        instructions.textContent = assignment.submission_instructions;
        instructionsWrap.style.display = "block";
    } else {
        instructions.textContent = "";
        instructionsWrap.style.display = "none";
    }

    resetAssignmentDropbox(assignment);
    setAssignmentModalTab(isInstructor && initialTab === "dropbox" ? "submissions" : initialTab);
    document.getElementById("assignment-details-modal").style.display = "flex";
}

function closeAssignmentDetails() {
    currentAssignmentDetailsId = null;
    document.getElementById("assignment-details-modal").style.display = "none";
}

function resetAssignmentDropbox(assignment) {
    const fileInput = document.getElementById("assignment-dropbox-file-input");
    const noteInput = document.getElementById("assignment-dropbox-note-input");
    const status = document.getElementById("assignment-dropbox-status");
    const submitButton = document.getElementById("submit-assignment-dropbox-btn");
    const existing = document.getElementById("assignment-dropbox-existing");
    const acceptsFiles = assignment.allow_file_submission ?? true;

    if (fileInput) {
        fileInput.value = "";
        fileInput.disabled = !acceptsFiles;
        fileInput.accept = getAssignmentFileAccept(assignment.expected_file_type);
    }

    if (noteInput) {
        noteInput.value = "";
        noteInput.disabled = assignment.allow_text_submission === false;
    }

    if (status) {
        status.textContent = acceptsFiles
            ? ""
            : "This assignment is not accepting file uploads.";
    }

    if (submitButton) {
        submitButton.disabled = !acceptsFiles;
        submitButton.textContent = "Submit Assignment";
    }

    if (existing) {
        existing.innerHTML = '<span>No submissions uploaded yet.</span>';
    }
}

function getAssignmentFileAccept(fileType) {
    const accepts = {
        pdf: ".pdf,application/pdf",
        docx: ".docx,application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        pptx: ".pptx,application/vnd.openxmlformats-officedocument.presentationml.presentation",
        xlsx: ".xlsx,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        zip: ".zip,application/zip,application/x-zip-compressed",
        image: "image/*",
    };

    return accepts[fileType] || "";
}

function getFileExtension(fileName) {
    const parts = String(fileName || "").toLowerCase().split(".");
    return parts.length > 1 ? parts.pop() : "";
}

function doesFileMatchExpectedType(file, expectedFileType) {
    if (!expectedFileType || !file) {
        return true;
    }

    const extension = getFileExtension(file.name);
    const contentType = String(file.type || "").toLowerCase();

    const extensionMatches = {
        pdf: extension === "pdf",
        docx: extension === "docx",
        pptx: extension === "pptx",
        xlsx: extension === "xlsx",
        zip: extension === "zip",
        image: ["jpg", "jpeg", "png", "gif", "webp", "bmp", "svg"].includes(extension),
    };

    const contentTypeMatches = {
        pdf: contentType === "application/pdf",
        docx: contentType === "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        pptx: contentType === "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        xlsx: contentType === "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        zip: contentType === "application/zip" || contentType === "application/x-zip-compressed",
        image: contentType.startsWith("image/"),
    };

    return Boolean(extensionMatches[expectedFileType]) &&
        (!contentType || Boolean(contentTypeMatches[expectedFileType]));
}

function validateDropboxFile(assignment, file) {
    if (!file) {
        return null;
    }

    if (assignment.expected_file_type && !doesFileMatchExpectedType(file, assignment.expected_file_type)) {
        return `File type must match ${getFileTypeLabel(assignment.expected_file_type)}.`;
    }

    if (assignment.max_file_size_mb && file.size > Number(assignment.max_file_size_mb) * 1024 * 1024) {
        return `File must be ${assignment.max_file_size_mb} MB or smaller.`;
    }

    return null;
}

function renderSubmissionHistory(submissions) {
    if (!submissions.length) {
        return '<span>No submissions uploaded yet.</span>';
    }

    return `
        <strong>Past submissions</strong>
        <div class="dropbox-history-list">
            ${submissions.map((submission, index) => {
                const submittedAt = formatAssignmentDate(submission.submitted_at);
                const fileLink = submission.file_url
                    ? `<a href="${escapeHtml(submission.file_url)}" target="_blank" rel="noopener">Open submitted file</a>`
                    : "<span>No file attached.</span>";
                const note = submission.submission_text
                    ? `<p class="dropbox-history-note">${escapeHtml(submission.submission_text)}</p>`
                    : "";
                const grade = submission.score !== null && submission.score !== undefined
                    ? `<span class="dropbox-history-pill">Score: ${escapeHtml(formatGradeNumber(submission.score))}</span>`
                    : index === 0
                        ? `<span class="dropbox-history-pill pending">Pending grade</span>`
                        : "";
                const feedback = submission.feedback
                    ? `<p class="dropbox-history-note"><strong>Feedback:</strong> ${escapeHtml(submission.feedback)}</p>`
                    : "";

                return `
                    <div class="dropbox-history-item">
                        <div class="dropbox-history-head">
                            <span>Submission ${submissions.length - index}</span>
                            ${grade}
                        </div>
                        <span class="dropbox-history-date">Submitted: ${escapeHtml(submittedAt)}</span>
                        ${fileLink}
                        ${note}
                        ${feedback}
                    </div>
                `;
            }).join("")}
        </div>
    `;
}

async function loadAssignmentSubmissions(assignmentId) {
    const existing = document.getElementById("assignment-dropbox-existing");

    if (!existing) {
        return;
    }

    existing.innerHTML = "<span>Checking past submissions...</span>";

    try {
        const response = await axios.get(`/api/assignments/${assignmentId}/submissions/my`);
        existing.innerHTML = renderSubmissionHistory(response.data || []);
    } catch (error) {
        existing.innerHTML = '<span>Past submissions could not be loaded.</span>';
    }
}

function renderStaffSubmissionList(submissions) {
    if (!submissions.length) {
        return '<p class="grades-empty">No student submissions yet.</p>';
    }

    return `
        <div class="staff-submission-list">
            ${submissions.map((submission) => {
                const submittedAt = formatAssignmentDate(submission.submitted_at);
                const score = submission.score ?? "";
                const feedback = submission.feedback || "";
                const fileLink = submission.file_url
                    ? `<a href="${escapeHtml(submission.file_url)}" target="_blank" rel="noopener">Open submitted file</a>`
                    : "<span>No file attached.</span>";
                const note = submission.submission_text
                    ? `<p class="dropbox-history-note">${escapeHtml(submission.submission_text)}</p>`
                    : "";
                const latestBadge = submission.is_latest
                    ? '<span class="dropbox-history-pill">Latest</span>'
                    : '<span class="dropbox-history-pill pending">Past submission</span>';
                const gradeControls = submission.is_latest
                    ? `
                        <div class="staff-grade-form">
                            <label>
                                Score
                                <input id="grade-score-${submission.submission_id}" type="number" min="0" step="0.01" value="${escapeHtml(score)}">
                            </label>
                            <label>
                                Feedback
                                <textarea id="grade-feedback-${submission.submission_id}" rows="2">${escapeHtml(feedback)}</textarea>
                            </label>
                            <div class="staff-grade-actions">
                                <button type="button" onclick="saveSubmissionGrade(${submission.submission_id})">Save Grade</button>
                                <button type="button" class="danger-btn" onclick="clearSubmissionGrade(${submission.submission_id})">Clear Grade</button>
                            </div>
                        </div>
                    `
                    : `
                        <div class="staff-grade-readonly">
                            ${score !== "" ? `<span>Score: ${escapeHtml(formatGradeNumber(score))}</span>` : "<span>Not graded</span>"}
                            ${feedback ? `<p class="dropbox-history-note"><strong>Feedback:</strong> ${escapeHtml(feedback)}</p>` : ""}
                        </div>
                    `;

                return `
                    <div class="staff-submission-item">
                        <div class="staff-submission-head">
                            <div>
                                <strong>${escapeHtml(submission.student_name || "Student")}</strong>
                                <span>${escapeHtml(submission.student_email || "")}</span>
                            </div>
                            <div class="staff-submission-meta">
                                ${latestBadge}
                                <span class="dropbox-history-date">Submitted: ${escapeHtml(submittedAt)}</span>
                            </div>
                        </div>
                        ${fileLink}
                        ${note}
                        ${gradeControls}
                    </div>
                `;
            }).join("")}
        </div>
    `;
}

async function loadStaffAssignmentSubmissions(assignmentId) {
    const list = document.getElementById("assignment-submissions-list");

    if (!list) {
        return;
    }

    list.innerHTML = '<p class="grades-empty">Loading student submissions...</p>';

    try {
        const response = await axios.get(`/api/assignments/${assignmentId}/submissions`);
        list.innerHTML = renderStaffSubmissionList(response.data || []);
    } catch (error) {
        const message = error.response?.data || "Failed to load student submissions.";
        list.innerHTML = `<p class="grades-error">${escapeHtml(message)}</p>`;
    }
}

async function saveSubmissionGrade(submissionId) {
    const scoreInput = document.getElementById(`grade-score-${submissionId}`);
    const feedbackInput = document.getElementById(`grade-feedback-${submissionId}`);

    if (!scoreInput || !feedbackInput) {
        return;
    }

    const score = scoreInput.value.trim();

    if (score === "" || Number(score) < 0) {
        alert("Please enter a score of 0 or higher");
        return;
    }

    try {
        await axios.put(`/api/submissions/${submissionId}/grade`, {
            score: Number(score),
            feedback: feedbackInput.value.trim() || null,
        });

        if (currentAssignmentDetailsId) {
            await loadStaffAssignmentSubmissions(currentAssignmentDetailsId);
        }

        gradesLoaded = false;
        showActionMessage("Grade saved.", "success");
    } catch (error) {
        showActionMessage(error.response?.data || "Failed to save grade.", "error");
    }
}

async function clearSubmissionGrade(submissionId) {
    if (!confirm("Clear this grade?")) {
        return;
    }

    try {
        await axios.delete(`/api/submissions/${submissionId}/grade`);

        if (currentAssignmentDetailsId) {
            await loadStaffAssignmentSubmissions(currentAssignmentDetailsId);
        }

        gradesLoaded = false;
        showActionMessage("Grade cleared.", "success");
    } catch (error) {
        showActionMessage(error.response?.data || "Failed to clear grade.", "error");
    }
}

async function loadCourseTitle() {
    try {
        const response = await axios.get("/api/course/" + courseId);
        currentCourse = response.data;

        refreshCourseDisplay();
    } catch (error) {
        console.error("Failed to load course title:", error);
        showActionMessage("Failed to load course details.", "error");
    }
}

async function loadManageAccess() {
    try {
        const response = await axios.get(`/api/courses/${courseId}/manage-access`);
        isInstructor = Boolean(response.data.can_manage);

        const heroActions = document.getElementById("course-hero-actions");
        if (heroActions) {
            heroActions.style.display = isInstructor ? "flex" : "none";
        }

        setModuleCardAddVisible(isInstructor);
        setAssignmentCardAddVisible(isInstructor);
        setQuizCardAddVisible(isInstructor);

        const actionStrip = document.querySelector(".course-action-strip");
        if (actionStrip) {
            actionStrip.style.display = isInstructor ? "none" : "grid";
        }

        setGradeTabsVisible(!isInstructor);
        if (isInstructor) {
            hideCourseProgress();
        }
    } catch (error) {
        isInstructor = false;
        const heroActions = document.getElementById("course-hero-actions");
        if (heroActions) {
            heroActions.style.display = "none";
        }
        const actionStrip = document.querySelector(".course-action-strip");
        if (actionStrip) {
            actionStrip.style.display = "grid";
        }
        setModuleCardAddVisible(false);
        setAssignmentCardAddVisible(false);
        setQuizCardAddVisible(false);
        setGradeTabsVisible(true);
        console.error("Failed to load course management access:", error);
    }
}

function setModuleCardAddVisible(visible) {
    const moduleCardAddButton = document.getElementById("add-module-btn");

    if (moduleCardAddButton) {
        moduleCardAddButton.style.display = visible ? "inline-flex" : "none";
    }
}

function setAssignmentCardAddVisible(visible) {
    const assignmentCardAddButton = document.getElementById("assignment-card-add-btn");

    if (assignmentCardAddButton) {
        assignmentCardAddButton.style.display = visible ? "inline-flex" : "none";
    }
}

function setQuizCardAddVisible(visible) {
    const quizCardAddButton = document.getElementById("quiz-card-add-btn");

    if (quizCardAddButton) {
        quizCardAddButton.style.display = visible ? "inline-flex" : "none";
    }
}

function populateDueTimeOptions() {
    const timeSelect = document.getElementById("assignment-due-time-input");

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

function setAssignmentSaveState(isSaving, message = "") {
    const saveButton = document.getElementById("save-assignment-btn");
    const closeButton = document.getElementById("close-assignment-modal-btn");
    const status = document.getElementById("assignment-save-status");

    if (saveButton) {
        saveButton.disabled = isSaving;
        saveButton.textContent = isSaving ? "Saving..." : "Save";
    }

    if (closeButton) {
        closeButton.disabled = isSaving;
    }

    if (status) {
        status.textContent = message;
    }
}

function formatAssignmentDate(value) {
    if (!value) {
        return "No due date";
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

function getSingaporeDateTimeParts(value) {
    const date = parseApiDateTime(value);

    if (Number.isNaN(date.getTime())) {
        return null;
    }

    const parts = new Intl.DateTimeFormat("en-SG", {
        timeZone: SG_TIME_ZONE,
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
        hour12: false,
    }).formatToParts(date);

    return Object.fromEntries(parts.map((part) => [part.type, part.value]));
}

function parseApiDateTime(value) {
    if (typeof value !== "string") {
        return new Date(value);
    }

    const normalizedValue = value.includes("T") ? value : value.replace(" ", "T");
    const hasTimezone = TIMEZONE_OFFSET_PATTERN.test(normalizedValue);

    return new Date(hasTimezone ? normalizedValue : `${normalizedValue}Z`);
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

function toDatetimeLocalValue(value) {
    if (!value) {
        return "";
    }

    const singaporeParts = getSingaporeDateTimeParts(value);

    if (!singaporeParts) {
        return value.slice(0, 16);
    }

    return `${singaporeParts.year}-${singaporeParts.month}-${singaporeParts.day}T${singaporeParts.hour}:${singaporeParts.minute}`;
}

function getDateInputValue(value) {
    return toDatetimeLocalValue(value).slice(0, 10);
}

function getTimeInputValue(value) {
    const localValue = toDatetimeLocalValue(value);
    return localValue.length >= 16 ? localValue.slice(11, 16) : "00:00";
}

function toApiDateTime(value) {
    if (!value) {
        return null;
    }

    const date = new Date(`${value}:00+08:00`);

    if (Number.isNaN(date.getTime())) {
        return `${value}:00`;
    }

    return date.toISOString().slice(0, 19);
}

function getAssignmentSubmissionLabel(assignment) {
    const methods = [];

    if (assignment.allow_text_submission ?? true) {
        methods.push("Text");
    }

    if (assignment.allow_file_submission ?? true) {
        methods.push("File");
    }

    return methods.length ? methods.join(" and ") : "No submission method set";
}

function getFileTypeLabel(value) {
    const labels = {
        pdf: "PDF",
        docx: "Word document (.docx)",
        pptx: "PowerPoint (.pptx)",
        xlsx: "Excel spreadsheet (.xlsx)",
        zip: "ZIP archive",
        image: "Image",
    };

    return labels[value] || "Any file type";
}

function editCourse(event, courseId) {
    event.stopPropagation();
    openCourseModal();
}

async function deleteCourse(event, courseId) {
    event.stopPropagation();

    if (!confirm("Delete this course?")) {
        return;
    }

    try {
        await axios.delete(`/api/courses/${courseId}`);
        window.location.href = "/courses";
    } catch (error) {
        const message = error.response?.data || "Failed to delete course.";
        showActionMessage(message, "error");
    }
}

function updateCoursePaidFields() {
    const isPaid = document.getElementById("course-paid-input").checked;
    document.getElementById("course-paid-fields").hidden = !isPaid;
}

function openCourseModal() {
    if (!currentCourse) {
        return;
    }

    document.getElementById("course-name-input").value = currentCourse.name || "";
    document.getElementById("course-name-input").placeholder = currentCourse.name || "Course name";
    document.getElementById("course-description-input").value = currentCourse.description || "";
    document.getElementById("course-description-input").placeholder = currentCourse.description || "Course description";
    document.getElementById("course-image-input").value = "";
    const priceCents = getCoursePriceCents(currentCourse);
    document.getElementById("course-price-input").value =
        priceCents === null ? "" : (priceCents / 100).toFixed(2);
    document.getElementById("course-price-input").placeholder =
        priceCents === null ? "0.00" : (priceCents / 100).toFixed(2);
    document.getElementById("course-currency-input").value = (currentCourse.currency || "SGD").toUpperCase();
    document.getElementById("course-currency-input").placeholder = currentCourse.currency || "SGD";
    document.getElementById("course-status-input").value = currentCourse.status || "draft";
    document.getElementById("course-paid-input").checked = Boolean(currentCourse.is_paid);
    updateCoursePaidFields();
    document.getElementById("edit-course-modal").style.display = "flex";
}

function closeCourseModal() {
    document.getElementById("edit-course-modal").style.display = "none";
}

async function uploadCourseImage(file) {
    const formData = new FormData();
    formData.append("file", file);
    formData.append("folder", "lms/courses");

    const response = await axios.post("/api/cloudinary/upload", formData, {
        headers: {
            "Content-Type": "multipart/form-data",
        },
    });

    return response.data.secure_url;
}

async function uploadAssignmentBrief(file) {
    const formData = new FormData();
    formData.append("file", file);
    formData.append("folder", "lms/assignments");

    const response = await axios.post("/api/cloudinary/upload", formData, {
        headers: {
            "Content-Type": "multipart/form-data",
        },
    });

    return response.data.secure_url;
}

async function uploadSubmissionFile(file) {
    const formData = new FormData();
    formData.append("file", file);
    formData.append("folder", "lms/submissions");

    const response = await axios.post("/api/cloudinary/upload", formData, {
        headers: {
            "Content-Type": "multipart/form-data",
        },
    });

    return response.data;
}

function setDropboxSubmitState(isSaving, message = "") {
    const submitButton = document.getElementById("submit-assignment-dropbox-btn");
    const status = document.getElementById("assignment-dropbox-status");

    if (submitButton) {
        submitButton.disabled = isSaving;
        submitButton.textContent = isSaving ? "Submitting..." : "Submit Assignment";
    }

    if (status) {
        status.textContent = message;
    }
}

async function submitAssignmentDropbox() {
    if (!currentAssignmentDetailsId) {
        return;
    }

    const assignment = currentAssignments.find((item) => item.assignment_id === currentAssignmentDetailsId);
    const fileInput = document.getElementById("assignment-dropbox-file-input");
    const noteInput = document.getElementById("assignment-dropbox-note-input");
    const file = fileInput?.files?.[0];
    const note = noteInput?.value.trim() || null;

    if (!assignment) {
        return;
    }

    if ((assignment.allow_file_submission ?? true) && !file) {
        setDropboxSubmitState(false, "Please choose a file to upload.");
        return;
    }

    const fileError = validateDropboxFile(assignment, file);

    if (fileError) {
        setDropboxSubmitState(false, fileError);
        return;
    }

    try {
        setDropboxSubmitState(true, file ? "Uploading file..." : "Submitting...");

        const upload = file ? await uploadSubmissionFile(file) : null;
        setDropboxSubmitState(true, "Saving submission...");

        await axios.post(`/api/assignments/${currentAssignmentDetailsId}/submissions`, {
            submission_text: note,
            file_url: upload?.secure_url || null,
            cloudinary_public_id: upload?.public_id || null,
            file_name: file?.name || null,
            file_content_type: file?.type || null,
            file_size: file?.size || null,
        });

        gradesLoaded = false;
        setDropboxSubmitState(false, "Assignment submitted.");
        await loadAssignmentSubmissions(currentAssignmentDetailsId);
    } catch (error) {
        if (error.response?.status === 401) {
            window.location.href = "/login";
            return;
        }

        const message = error.response?.data || "Failed to submit assignment.";
        setDropboxSubmitState(false, message);
    }
}

async function saveCourse() {
    const name = document.getElementById("course-name-input").value.trim();
    const description = document.getElementById("course-description-input").value.trim();
    const backgroundImageFile = document.getElementById("course-image-input").files[0];
    const priceInputValue = document.getElementById("course-price-input").value.trim();
    const currency = document.getElementById("course-currency-input").value.trim() || "SGD";
    const status = document.getElementById("course-status-input").value;
    const isPaid = document.getElementById("course-paid-input").checked;

    if (!name) {
        alert("Please enter a course name");
        return;
    }

    const price = Number(priceInputValue);

    if (isPaid && (priceInputValue === "" || !Number.isFinite(price) || price <= 0)) {
        showActionMessage("Paid courses must have a price greater than zero.", "error");
        return;
    }

    try {
        const backgroundImageUrl = backgroundImageFile
            ? await uploadCourseImage(backgroundImageFile)
            : currentCourse.background_image_url;

        const payload = {
            name,
            description: description || null,
            background_image_url: backgroundImageUrl || null,
            currency: isPaid ? currency : "SGD",
            status,
            is_paid: isPaid,
        };

        if (!isPaid) {
            payload.price = 0;
        } else {
            payload.price = price;
        }

        await axios.put(`/api/courses/${courseId}`, payload);

        closeCourseModal();
        await loadCourseTitle();
        showActionMessage("Course updated.", "success");
    } catch (error) {
        const message = error.response?.data || "Failed to update course.";
        showActionMessage(message, "error");
    }
}

function editModule(event, moduleId) {
    event.stopPropagation();
    const module = currentModules.find((item) => item.module_id === moduleId);

    if (!module) {
        return;
    }

    openModuleModal(module);
}

async function deleteModule(event, moduleId) {
    event.stopPropagation();

    if (!confirm("Delete this module?")) {
        return;
    }

    await axios.delete(`/api/module/${moduleId}`);
    loadModules();
}

function openModuleModal(module = null) {
    currentEditingModuleId = module?.module_id || null;
    document.getElementById("module-modal-title").textContent = module ? "Edit Module" : "Add Module";
    document.getElementById("module-title-input").value = module?.title || "";
    document.getElementById("module-title-input").placeholder = module?.title || "Module title, e.g. Week 1 Introduction";
    document.getElementById("module-position-input").value =
        module?.position || currentModules.length + 1;
    document.getElementById("module-position-input").placeholder =
        String(module?.position || currentModules.length + 1);
    document.getElementById("add-module-modal").style.display = "flex";
}

function closeModuleModal() {
    currentEditingModuleId = null;
    document.getElementById("module-title-input").value = "";
    document.getElementById("module-title-input").placeholder = "Module title, e.g. Week 1 Introduction";
    document.getElementById("module-position-input").value = "";
    document.getElementById("module-position-input").placeholder = "1";
    document.getElementById("add-module-modal").style.display = "none";
}

function openAssignmentModal(assignment = null) {
    populateDueTimeOptions();
    setAssignmentSaveState(false, "");
    currentEditingAssignmentId = assignment?.assignment_id || null;
    currentAssignmentBriefUrl = assignment?.assignment_brief_url || null;
    document.getElementById("assignment-modal-title").textContent = assignment ? "Edit Assignment" : "Add Assignment";
    document.getElementById("assignment-title-input").value = assignment?.title || "";
    document.getElementById("assignment-description-input").value = assignment?.description || "";
    document.getElementById("assignment-due-date-input").value = getDateInputValue(assignment?.due_date);
    document.getElementById("assignment-due-time-input").value = getTimeInputValue(assignment?.due_date);
    document.getElementById("assignment-score-input").value = assignment?.max_score ?? "";
    document.getElementById("assignment-brief-file-input").value = "";
    const briefLink = document.getElementById("assignment-brief-current-link");
    if (briefLink) {
        briefLink.href = currentAssignmentBriefUrl || "#";
        briefLink.style.display = currentAssignmentBriefUrl ? "inline-flex" : "none";
    }
    document.getElementById("assignment-file-type-input").value = assignment?.expected_file_type || "";
    document.getElementById("assignment-text-input").checked = assignment?.allow_text_submission ?? true;
    document.getElementById("assignment-file-input").checked = assignment?.allow_file_submission ?? true;
    document.getElementById("assignment-file-size-input").value = assignment?.max_file_size_mb ?? "";
    document.getElementById("assignment-instructions-input").value = assignment?.submission_instructions || "";
    document.getElementById("assignment-modal").style.display = "flex";
}

function closeAssignmentModal() {
    currentEditingAssignmentId = null;
    currentAssignmentBriefUrl = null;
    document.getElementById("assignment-modal").style.display = "none";
}

function editQuiz(event, quizId) {
    event.stopPropagation();
    window.location.href = `/course/${courseId}/quiz-builder?quiz_id=${quizId}`;
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

function editAssignment(event, assignmentId) {
    event.stopPropagation();
    const assignment = currentAssignments.find((item) => item.assignment_id === assignmentId);

    if (!assignment) {
        return;
    }

    openAssignmentModal(assignment);
}

async function deleteAssignment(event, assignmentId) {
    event.stopPropagation();

    if (!confirm("Delete this assignment?")) {
        return;
    }

    try {
        await axios.delete(`/api/assignment/${assignmentId}`);
        await loadAssignments();
        showActionMessage("Assignment deleted.", "success");
    } catch (error) {
        const message = error.response?.data || "Failed to delete assignment.";
        showActionMessage(message, "error");
    }
}

async function saveAssignment() {
    const title = document.getElementById("assignment-title-input").value.trim();
    const description = document.getElementById("assignment-description-input").value.trim();
    const dueDate = document.getElementById("assignment-due-date-input").value;
    const dueTime = document.getElementById("assignment-due-time-input").value;
    const maxScore = document.getElementById("assignment-score-input").value.trim();
    const briefFile = document.getElementById("assignment-brief-file-input").files[0];
    const expectedFileType = document.getElementById("assignment-file-type-input").value;
    const maxFileSize = document.getElementById("assignment-file-size-input").value.trim();
    const submissionInstructions = document.getElementById("assignment-instructions-input").value.trim();

    if (!title) {
        alert("Please enter an assignment title");
        return;
    }

    if (!description) {
        alert("Please enter an assignment description");
        return;
    }

    if (!dueDate) {
        alert("Please choose a due date");
        return;
    }

    if (maxScore === "" || Number(maxScore) < 0) {
        alert("Please enter a max score of 0 or higher");
        return;
    }

    try {
        setAssignmentSaveState(true, briefFile ? "Uploading assignment brief..." : "Saving assignment...");

        let briefUrl = currentAssignmentBriefUrl;

        if (briefFile) {
            briefUrl = await uploadAssignmentBrief(briefFile);
            setAssignmentSaveState(true, "Brief uploaded. Saving assignment...");
        }

        const payload = {
            course_id: Number(courseId),
            title,
            description,
            due_date: toApiDateTime(`${dueDate}T${dueTime}`),
            max_score: Number(maxScore),
            assignment_brief_url: briefUrl || null,
            expected_file_type: expectedFileType || null,
            allow_text_submission: document.getElementById("assignment-text-input").checked,
            allow_file_submission: document.getElementById("assignment-file-input").checked,
            max_file_size_mb: maxFileSize ? Number(maxFileSize) : null,
            submission_instructions: submissionInstructions || null,
        };

        if (currentEditingAssignmentId) {
            await axios.put(`/api/assignment/${currentEditingAssignmentId}`, payload);
        } else {
            await axios.post("/api/assignment", payload);
        }

        closeAssignmentModal();
        await loadAssignments();
        showActionMessage("Assignment saved.", "success");
    } catch (error) {
        const message = error.response?.data || "Failed to save assignment.";
        setAssignmentSaveState(false, "");
        showActionMessage(message, "error");
    }
}

async function saveModule() {
    const title = document.getElementById("module-title-input").value.trim();
    const position = Number(document.getElementById("module-position-input").value || 0);

    if (!title) {
        alert("Please enter a module title");
        return;
    }

    if (!Number.isInteger(position) || position < 1) {
        alert("Please enter a display order of 1 or higher");
        return;
    }

    if (currentEditingModuleId) {
        const currentModule = currentModules.find((module) => module.module_id === currentEditingModuleId);

        if (currentModule && currentModule.position !== position) {
            const confirmed = confirm(`Do you want to move ${title} to order ${position}?`);

            if (!confirmed) {
                return;
            }
        }

        await axios.put(`/api/modules/${currentEditingModuleId}`, {
            title,
            position,
        });
    } else {
        await axios.post("/api/modules", {
            course_id: Number(courseId),
            title,
            position,
        });
    }

    closeModuleModal();
    await loadModules();
}

function bindInstructorControls() {
    document.getElementById("edit-course-btn")?.addEventListener("click", (event) => {
        editCourse(event, courseId);
    });

    document.getElementById("delete-course-btn")?.addEventListener("click", (event) => {
        deleteCourse(event, courseId);
    });

    document.getElementById("save-course-btn")?.addEventListener("click", saveCourse);
    document.getElementById("close-course-modal-btn")?.addEventListener("click", closeCourseModal);
    document.getElementById("course-paid-input")?.addEventListener("change", updateCoursePaidFields);

    document.getElementById("student-view-btn")?.addEventListener("click", async () => {
        const heroActions = document.getElementById("course-hero-actions");
        if (heroActions) {
            heroActions.style.display = "none";
        }
        isInstructor = false;
        setModuleCardAddVisible(false);
        setAssignmentCardAddVisible(false);
        setQuizCardAddVisible(false);
        setGradeTabsVisible(true);
        await loadCourseModuleProgresses();
        loadModules();
        loadAssignments();
        loadQuizzes();
        loadCourseProgress();
    });

    document.getElementById("add-module-btn")?.addEventListener("click", () => {
        openModuleModal();
    });

<<<<<<< HEAD
=======
    document.getElementById("add-assignment-btn")?.addEventListener("click", () => {
        openAssignmentModal();
    });

    document.getElementById("add-quiz-btn")?.addEventListener("click", () => {
        window.location.href = `/course/${courseId}/quiz-builder`;
    });

>>>>>>> 25d6b41887e346fb6c7826e217af1e82a902e9df
    document.getElementById("assignment-card-add-btn")?.addEventListener("click", () => {
        openAssignmentModal();
    });

    document.getElementById("quiz-card-add-btn")?.addEventListener("click", () => {
        window.location.href = `/course/${courseId}/quiz-builder`;
    });

    document.getElementById("close-module-modal-btn")?.addEventListener("click", () => {
        closeModuleModal();
    });

    document.getElementById("save-module-btn")?.addEventListener("click", saveModule);
    document.getElementById("save-assignment-btn")?.addEventListener("click", saveAssignment);
    document.getElementById("close-assignment-modal-btn")?.addEventListener("click", closeAssignmentModal);
    document.getElementById("close-assignment-details-btn")?.addEventListener("click", closeAssignmentDetails);
    document.getElementById("submit-assignment-dropbox-btn")?.addEventListener("click", submitAssignmentDropbox);
    document.getElementById("refresh-grades-btn")?.addEventListener("click", () => {
        gradesLoaded = false;
        loadGrades();
    });

    document.querySelectorAll(".course-tab").forEach((tab) => {
        tab.addEventListener("click", () => {
            setActiveCourseTab(tab.dataset.courseTab);
        });
    });

    document.querySelectorAll(".assignment-modal-tab").forEach((tab) => {
        tab.addEventListener("click", () => {
            setAssignmentModalTab(tab.dataset.assignmentTab);
        });
    });
}

async function init() {
    document.getElementById("course-action-button")
        ?.addEventListener("click", handleCourseAction);

    bindInstructorControls();

    await loadCourseTitle();
    await loadEnrollmentStatus();
    await loadManageAccess();
    await loadCourseProgress();
    await loadCourseModuleProgresses();
    await loadModules();
    await loadAssignments();
    await loadQuizzes();
}

init();
