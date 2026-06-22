const pathParts = window.location.pathname.split("/");
const courseId = pathParts[2];
const SG_TIME_ZONE = "Asia/Singapore";
const TIMEZONE_OFFSET_PATTERN = /(Z|[+-]\d{2}:?\d{2})$/i;
const COURSE_IMAGE_PRESETS = [
    { title: "Software Development", url: "https://images.unsplash.com/photo-1515879218367-8466d910aaa4" },
    { title: "Web Development", url: "https://images.unsplash.com/photo-1461749280684-dccba630e2f6" },
    { title: "Mobile Development", url: "https://images.unsplash.com/photo-1512941937669-90a1b58e7e9c" },
    { title: "Game Development", url: "https://images.unsplash.com/photo-1542751371-adc38448a05e" },
    { title: "Data Analytics", url: "https://images.unsplash.com/photo-1551288049-bebda4e38f71" },
    { title: "Data Science", url: "https://images.unsplash.com/photo-1527474305487-b87b222841cc" },
    { title: "Artificial Intelligence", url: "https://images.unsplash.com/photo-1677442136019-21780ecad995" },
    { title: "Machine Learning", url: "https://images.unsplash.com/photo-1485827404703-89b55fcc595e" },
    { title: "Cybersecurity", url: "https://images.unsplash.com/photo-1563986768609-322da13575f3" },
    { title: "Cloud Computing", url: "https://images.unsplash.com/photo-1451187580459-43490279c0fa" },
    { title: "DevOps", url: "https://images.unsplash.com/photo-1558494949-ef010cbdcc31" },
    { title: "Blockchain", url: "https://images.unsplash.com/photo-1639762681485-074b7f938ba0" },
    { title: "Business Management", url: "https://images.unsplash.com/photo-1552664730-d307ca884978" },
    { title: "Project Management", url: "https://images.unsplash.com/photo-1454165804606-c3d57bc86b40" },
    { title: "Leadership", url: "https://images.unsplash.com/photo-1522202176988-66273c2fd55f" },
    { title: "Entrepreneurship", url: "https://images.unsplash.com/photo-1559136555-9303baea8ebd" },
    { title: "Human Resources", url: "https://images.unsplash.com/photo-1521737604893-d14cc237f11d" },
    { title: "Finance", url: "https://images.unsplash.com/photo-1520607162513-77705c0f0d4a" },
    { title: "Accounting", url: "https://images.unsplash.com/photo-1554224155-6726b3ff858f" },
    { title: "Investment", url: "https://images.unsplash.com/photo-1611974789855-9c2a0a7236a3" },
    { title: "Digital Marketing", url: "https://images.unsplash.com/photo-1460925895917-afdab827c52f" },
    { title: "Content Marketing", url: "https://images.unsplash.com/photo-1432888622747-4eb9a8efeb07" },
    { title: "Social Media Marketing", url: "https://images.unsplash.com/photo-1611162616475-46b635cb6868" },
    { title: "Sales", url: "https://images.unsplash.com/photo-1556740749-887f6717d7e4" },
    { title: "Communication", url: "https://images.unsplash.com/photo-1515169067868-5387ec356754" },
    { title: "Public Speaking", url: "https://images.unsplash.com/photo-1475721027785-f74eccf877e2" },
    { title: "Design Thinking", url: "https://images.unsplash.com/photo-1504384308090-c894fdcc538d" },
    { title: "UI/UX Design", url: "https://images.unsplash.com/photo-1581291518857-4e27b48ff24e" },
    { title: "Graphic Design", url: "https://images.unsplash.com/photo-1626785774573-4b799315345d" },
    { title: "Photography", url: "https://images.unsplash.com/photo-1500530855697-b586d89ba3ee" },
    { title: "Video Editing", url: "https://images.unsplash.com/photo-1574717024653-61fd2cf4d44d" },
    { title: "Education", url: "https://images.unsplash.com/photo-1523050854058-8df90110c9f1" },
    { title: "Healthcare", url: "https://images.unsplash.com/photo-1576091160399-112ba8d25d1f" },
    { title: "Nursing", url: "https://images.unsplash.com/photo-1584515933487-779824d29309" },
    { title: "Psychology", url: "https://images.unsplash.com/photo-1506126613408-eca07ce68773" },
    { title: "Languages", url: "https://images.unsplash.com/photo-1546410531-bb4caa6b424d" },
    { title: "Engineering", url: "https://images.unsplash.com/photo-1581092919535-7146ff1a590e" },
    { title: "Hospitality", url: "https://images.unsplash.com/photo-1566073771259-6a8506099945" },
    { title: "Customer Service", url: "https://images.unsplash.com/photo-1551434678-e076c223a692" },
    { title: "Personal Development", url: "https://images.unsplash.com/photo-1517836357463-d25dfeac3438" }
];
const COURSE_IMAGE_RULES = {
    maxFileSizeBytes: 5 * 1024 * 1024,
    minWidth: 1200,
    minHeight: 675,
    targetRatio: 16 / 9,
    ratioTolerance: 0.08,
};
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
let currentQuizAttemptsQuizId = null;
let currentQuizAttemptRows = [];
let moduleProgressById = new Map();
let quizAttemptStatuses = {};
let selectedCoursePresetImage = null;
let selectedCourseImageFile = null;
let selectedCourseImageObjectUrl = null;
let courseOverviewRefreshPromise = null;
let hasCompletedInitialCourseOverviewLoad = false;

function goToModuleContent(moduleId) {
    const module = currentModules.find((item) => Number(item.module_id) === Number(moduleId));
    const prerequisite = module ? getFirstIncompleteModulePrerequisite(module) : null;

    if (!isInstructor && prerequisite) {
        showActionMessage(
            `Complete ${prerequisite.title || "the previous module"} before opening this module.`,
            "warning"
        );
        return;
    }

    window.location.href = "/module-content/" + moduleId;
}

function getModuleProgressPercent(moduleId) {
    const progress = moduleProgressById.get(Number(moduleId)) || {
        opened: false,
        progress_percent: 0,
    };

    return Math.max(0, Math.min(100, Number(progress.progress_percent || 0)));
}

function getFirstIncompleteModulePrerequisite(module) {
    if (isInstructor || !isEnrolled) {
        return null;
    }

    const prerequisiteIds = Array.isArray(module.prerequisite_module_ids)
        ? module.prerequisite_module_ids.map(Number)
        : [];

    return prerequisiteIds
        .map((moduleId) => currentModules.find((item) => Number(item.module_id) === moduleId))
        .filter(Boolean)
        .sort((first, second) => Number(first.position) - Number(second.position))
        .find((item) => getModuleProgressPercent(item.module_id) < 100) || null;
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
        await loadCourseOverview();
        await loadQuizAttemptStatuses();
        renderQuizzes(currentQuizzes);
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
        renderModules(response.data);
    } catch (error) {
        renderModulesError();
        console.error("Failed to load modules:", error);
    }
}

function renderModules(modules) {
    currentModules = (Array.isArray(modules) ? modules : [])
        .sort((first, second) => first.position - second.position);
    const moduleList = document.getElementById("module-list");

    if (!moduleList) {
        return;
    }

    moduleList.innerHTML = "";

    if (currentModules.length === 0) {
        moduleList.innerHTML = "<p>No modules available.</p>";
        return;
    }

    moduleList.innerHTML = currentModules.map((module) => {
        const percent = getModuleProgressPercent(module.module_id);
        const prerequisite = getFirstIncompleteModulePrerequisite(module);
        const isLocked = Boolean(prerequisite);
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
        const lockHint = isLocked
            ? `<div class="module-subtitle">Complete ${escapeHtml(prerequisite.title || "the previous module")} first</div>`
            : "";
        const rowClass = [
            "module-row",
            percent === 100 ? "completed" : "",
            isLocked ? "locked" : "",
        ].filter(Boolean).join(" ");

        return `
            <div class="${rowClass}" onclick="goToModuleContent(${module.module_id})">
                <div class="module-info">
                    <div class="module-title">${escapeHtml(module.title || "Untitled module")}</div>
                    ${lockHint}
                </div>
                ${instructorButtons}
                ${progressRing}
                <span class="module-arrow">${isLocked ? '<i class="bi bi-lock-fill" aria-hidden="true"></i>' : "&rsaquo;"}</span>
            </div>
        `;
    }).join("");
}

function renderModulesError() {
    currentModules = [];
    const moduleList = document.getElementById("module-list");

    if (moduleList) {
        moduleList.innerHTML = "<p>Unable to load modules right now.</p>";
    }
}

function showInitialCourseLoadingState() {
    const moduleList = document.getElementById("module-list");
    const assignmentList = document.getElementById("assignment-list");
    const quizList = document.getElementById("quiz-list");

    if (moduleList) {
        moduleList.innerHTML = '<p class="module-empty">Loading modules...</p>';
    }

    if (assignmentList) {
        assignmentList.innerHTML = '<p class="assignment-empty">Loading assignments...</p>';
    }

    if (quizList) {
        quizList.innerHTML = '<p class="quiz-empty">Loading quizzes...</p>';
    }
}

function applyCourseOverview(overview) {
    currentCourse = overview.course || null;
    isEnrolled = Boolean(overview.enrolled);
    isInstructor = Boolean(overview.can_manage);
    moduleProgressById = new Map(
        (Array.isArray(overview.module_progress) ? overview.module_progress : [])
            .map((progress) => [Number(progress.module_id), progress])
    );

    refreshCourseDisplay();

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

    if (overview.course_progress && !isInstructor) {
        renderCourseProgress(overview.course_progress);
    } else {
        hideCourseProgress();
    }

    renderModules(overview.modules || []);
    renderAssignments(overview.assignments || []);
    renderQuizzes(overview.quizzes || []);
}

async function loadCourseOverview() {
    const response = await axios.get(`/api/courses/${courseId}/overview`, {
        params: {
            _: Date.now(),
        },
        headers: {
            "Cache-Control": "no-cache",
        },
    });
    applyCourseOverview(response.data || {});
}

async function refreshCourseOverview() {
    if (courseOverviewRefreshPromise) {
        return courseOverviewRefreshPromise;
    }

    courseOverviewRefreshPromise = (async () => {
        await loadCourseOverview();
        await loadQuizAttemptStatuses();
        renderQuizzes(currentQuizzes);
        hasCompletedInitialCourseOverviewLoad = true;
    })();

    try {
        await courseOverviewRefreshPromise;
    } finally {
        courseOverviewRefreshPromise = null;
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
        loadQuizAnalyticsSummaries();
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

function buildGradeRow({ title, meta, score, maxScore, feedback, actionHtml = "" }) {
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
            ${actionHtml}
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

    const quizRows = quizzes.map((quiz) => {
        const canViewAnswers = quiz.is_graded && quiz.attempt_id;

        return buildGradeRow({
            title: quiz.title || "Untitled quiz",
            meta: getGradeDateLabel(quiz.submitted_at, "Submitted") || (quiz.attempt_id ? "Attempt in progress" : ""),
            score: quiz.total_score,
            maxScore: quiz.max_score,
            feedback: null,
            actionHtml: canViewAnswers
                ? `<button type="button" class="module-action-btn" onclick="openMyQuizAttemptReview(${quiz.attempt_id})">View Answers</button>`
                : "",
        });
    });

    gradeList.innerHTML =
        renderGradeSection("Assignments", assignmentRows, "No assignments are available for this course.") +
        renderGradeSection("Quizzes", quizRows, quizMessage || "No quizzes are available for this course.");
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

function clearCourseImageObjectUrl() {
    if (selectedCourseImageObjectUrl) {
        URL.revokeObjectURL(selectedCourseImageObjectUrl);
        selectedCourseImageObjectUrl = null;
    }
}

function setCourseImagePreview(imageUrl) {
    const preview = document.getElementById("course-image-preview");

    if (!preview) {
        return;
    }

    if (!imageUrl) {
        preview.style.backgroundImage = "";
        preview.innerHTML = "<span>No image selected</span>";
        return;
    }

    preview.style.backgroundImage = `url('${imageUrl}')`;
    preview.innerHTML = "";
}

function renderCourseImagePresets() {
    const grid = document.getElementById("course-image-preset-grid");

    if (!grid) {
        return;
    }

    grid.innerHTML = COURSE_IMAGE_PRESETS.map((preset, index) => `
        <button
            class="course-preset-option"
            type="button"
            data-preset-index="${index}"
            onclick="selectCoursePresetImage(${index})"
        >
            <span class="course-preset-thumb" style="background-image: url('${escapeHtml(preset.url)}')"></span>
            <span>${escapeHtml(preset.title)}</span>
        </button>
    `).join("");
}

function syncCoursePresetSelection() {
    document.querySelectorAll("#course-image-preset-grid .course-preset-option").forEach((button) => {
        const preset = COURSE_IMAGE_PRESETS[Number(button.dataset.presetIndex)];
        button.classList.toggle("selected", preset?.url === selectedCoursePresetImage?.url);
    });
}

function validateCourseImageSize(width, height) {
    if (width < COURSE_IMAGE_RULES.minWidth || height < COURSE_IMAGE_RULES.minHeight) {
        return `Image must be at least ${COURSE_IMAGE_RULES.minWidth} x ${COURSE_IMAGE_RULES.minHeight}.`;
    }

    if (Math.abs((width / height) - COURSE_IMAGE_RULES.targetRatio) > COURSE_IMAGE_RULES.ratioTolerance) {
        return "Image must be close to a 16:9 course cover shape.";
    }

    return "";
}

function selectCoursePresetImage(index) {
    const preset = COURSE_IMAGE_PRESETS[index];

    if (!preset || validateCourseImageSize(preset.width, preset.height)) {
        showActionMessage("Selected preset image does not fit the required cover size.", "error");
        return;
    }

    selectedCoursePresetImage = preset;
    selectedCourseImageFile = null;
    clearCourseImageObjectUrl();
    document.getElementById("course-image-input").value = "";
    setCourseImagePreview(preset.url);
    syncCoursePresetSelection();
}

async function getCourseImageDimensions(file) {
    const objectUrl = URL.createObjectURL(file);

    try {
        const image = new Image();
        const loaded = new Promise((resolve, reject) => {
            image.onload = () => resolve({
                width: image.naturalWidth,
                height: image.naturalHeight,
            });
            image.onerror = () => reject(new Error("Could not read image dimensions."));
        });

        image.src = objectUrl;
        return await loaded;
    } finally {
        URL.revokeObjectURL(objectUrl);
    }
}

async function validateCourseUploadedImage(file) {
    if (!file.type.startsWith("image/")) {
        return "Please upload an image file.";
    }

    if (file.size > COURSE_IMAGE_RULES.maxFileSizeBytes) {
        return "Image must be 5 MB or smaller.";
    }

    const dimensions = await getCourseImageDimensions(file);
    return validateCourseImageSize(dimensions.width, dimensions.height);
}

async function handleCourseImageFileChange(event) {
    const file = event.target.files?.[0] || null;

    if (!file) {
        selectedCourseImageFile = null;
        clearCourseImageObjectUrl();
        setCourseImagePreview(selectedCoursePresetImage?.url || currentCourse?.background_image_url || "");
        return;
    }

    const validationMessage = await validateCourseUploadedImage(file);

    if (validationMessage) {
        event.target.value = "";
        selectedCourseImageFile = null;
        showActionMessage(validationMessage, "error");
        return;
    }

    selectedCourseImageFile = file;
    selectedCoursePresetImage = null;
    syncCoursePresetSelection();
    clearCourseImageObjectUrl();
    selectedCourseImageObjectUrl = URL.createObjectURL(file);
    setCourseImagePreview(selectedCourseImageObjectUrl);
}

async function getSelectedCourseImageUrl() {
    if (selectedCourseImageFile) {
        return uploadCourseImage(selectedCourseImageFile);
    }

    return selectedCoursePresetImage?.url || currentCourse?.background_image_url || null;
}

function openCourseModal() {
    if (!currentCourse) {
        return;
    }

    selectedCourseImageFile = null;
    clearCourseImageObjectUrl();
    selectedCoursePresetImage = COURSE_IMAGE_PRESETS.find((preset) => {
        return preset.url === currentCourse.background_image_url;
    }) || null;

    document.getElementById("course-name-input").value = currentCourse.name || "";
    document.getElementById("course-name-input").placeholder = currentCourse.name || "Course name";
    document.getElementById("course-description-input").value = currentCourse.description || "";
    document.getElementById("course-description-input").placeholder = currentCourse.description || "Course description";
    document.getElementById("course-visibility-input").value = currentCourse.visibility || "public";
    document.getElementById("course-image-input").value = "";
    renderCourseImagePresets();
    syncCoursePresetSelection();
    setCourseImagePreview(currentCourse.background_image_url || "");
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
    selectedCoursePresetImage = null;
    selectedCourseImageFile = null;
    clearCourseImageObjectUrl();
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

async function saveCourse() {
    const name = document.getElementById("course-name-input").value.trim();
    const description = document.getElementById("course-description-input").value.trim();
    const priceInputValue = document.getElementById("course-price-input").value.trim();
    const currency = document.getElementById("course-currency-input").value.trim() || "SGD";
    const status = document.getElementById("course-status-input").value;
    const visibility = document.getElementById("course-visibility-input").value || "public";
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
        const backgroundImageUrl = await getSelectedCourseImageUrl();

        const payload = {
            name,
            description: description || null,
            background_image_url: backgroundImageUrl || null,
            currency: isPaid ? currency : "SGD",
            status,
            visibility,
            is_paid: isPaid,
        };

        if (!isPaid) {
            payload.price = 0;
        } else {
            payload.price = price;
        }

        await axios.put(`/api/courses/${courseId}`, payload);

        closeCourseModal();
        await loadCourseOverview();
        await loadQuizAttemptStatuses();
        renderQuizzes(currentQuizzes);
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
    populateModulePrerequisiteOptions(module);
    document.getElementById("add-module-modal").style.display = "flex";
}

function closeModuleModal() {
    currentEditingModuleId = null;
    document.getElementById("module-title-input").value = "";
    document.getElementById("module-title-input").placeholder = "Module title, e.g. Week 1 Introduction";
    document.getElementById("module-position-input").value = "";
    document.getElementById("module-position-input").placeholder = "1";
    const prerequisiteInput = document.getElementById("module-prerequisites-input");
    if (prerequisiteInput) {
        prerequisiteInput.innerHTML = "";
    }
    document.getElementById("add-module-modal").style.display = "none";
}

function populateModulePrerequisiteOptions(module = null) {
    const prerequisiteInput = document.getElementById("module-prerequisites-input");

    if (!prerequisiteInput) {
        return;
    }

    const selectedIds = new Set(
        Array.isArray(module?.prerequisite_module_ids)
            ? module.prerequisite_module_ids.map(Number)
            : []
    );
    const currentModuleId = Number(module?.module_id || 0);

    const options = currentModules
        .filter((item) => Number(item.module_id) !== currentModuleId)
        .sort((first, second) => Number(first.position) - Number(second.position))
        .map((item) => {
            const moduleId = Number(item.module_id);
            const checked = selectedIds.has(moduleId) ? "checked" : "";
            const label = `${item.position}. ${item.title || "Untitled module"}`;

            return `
                <label class="prerequisite-option">
                    <input type="checkbox" value="${moduleId}" ${checked}>
                    <span>${escapeHtml(label)}</span>
                </label>
            `;
        })
        .join("");

    prerequisiteInput.innerHTML = options || '<p class="prerequisite-empty">No other modules available.</p>';
}

function getSelectedModulePrerequisiteIds() {
    const prerequisiteInput = document.getElementById("module-prerequisites-input");

    if (!prerequisiteInput) {
        return [];
    }

    return [...prerequisiteInput.querySelectorAll('input[type="checkbox"]:checked')]
        .map((input) => Number(input.value));
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
            prerequisite_module_ids: getSelectedModulePrerequisiteIds(),
        });
    } else {
        await axios.post("/api/modules", {
            course_id: Number(courseId),
            title,
            position,
            prerequisite_module_ids: getSelectedModulePrerequisiteIds(),
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
    document.getElementById("course-image-input")?.addEventListener("change", handleCourseImageFileChange);

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
        moduleProgressById = new Map();
        hideCourseProgress();
        renderModules(currentModules);
        renderAssignments(currentAssignments);
        await loadQuizAttemptStatuses();
        renderQuizzes(currentQuizzes);
    });

    document.getElementById("add-module-btn")?.addEventListener("click", () => {
        openModuleModal();
    });

    document.getElementById("add-assignment-btn")?.addEventListener("click", () => {
        openAssignmentModal();
    });

    document.getElementById("add-quiz-btn")?.addEventListener("click", () => {
        window.location.href = `/course/${courseId}/quiz-builder`;
    });

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
    document.getElementById("close-quiz-attempts-btn")?.addEventListener("click", closeQuizAttempts);
    document.getElementById("close-quiz-analytics-btn")?.addEventListener("click", closeQuizAnalytics);
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
    showInitialCourseLoadingState();

    try {
        await refreshCourseOverview();
    } catch (error) {
        renderModulesError();
        renderAssignmentsError();
        renderQuizzesError();
        hideCourseProgress();
        showActionMessage("Failed to load course details.", "error");
        console.error("Failed to load course overview:", error);
    }
}

init();

window.addEventListener("pageshow", async () => {
    if (!hasCompletedInitialCourseOverviewLoad) {
        return;
    }

    sessionStorage.removeItem("skillup-course-progress-dirty");

    try {
        await refreshCourseOverview();
    } catch (error) {
        console.error("Failed to refresh course overview after returning to course details:", error);
    }
});

window.addEventListener("focus", async () => {
    if (!hasCompletedInitialCourseOverviewLoad) {
        return;
    }

    try {
        await refreshCourseOverview();
    } catch (error) {
        console.error("Failed to refresh course overview after focus:", error);
    }
});
