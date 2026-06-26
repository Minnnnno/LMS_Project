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
let completionRosterLoaded = false;
let completionRosterRows = [];
let discussionLoaded = false;
let discussionStatusMessageTimer = null;
let currentDiscussionTopics = [];
let currentDiscussionThreads = [];
let currentDiscussionTopic = null;
let currentDiscussionDetail = null;
let currentEditingTopicId = null;
let currentEditingThreadId = null;
let currentReplyParentId = null;
let currentDiscussionTopicPage = 1;
let currentDiscussionThreadPage = 1;
let currentDiscussionReplyPage = 1;
let currentDiscussionTopicPagination = null;
let currentDiscussionThreadPagination = null;
let currentDiscussionReplyPagination = null;
const DISCUSSION_TOPIC_PAGE_SIZE = 5;
const DISCUSSION_THREAD_PAGE_SIZE = 7;
const DISCUSSION_REPLY_PAGE_SIZE = 5;

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
    document.getElementById("course-discussion-panel")
        ?.classList.toggle("active", tabName === "discussion");
    document.getElementById("course-submissions-panel")
        ?.classList.toggle("active", tabName === "submissions");
    document.getElementById("course-completion-panel")
        ?.classList.toggle("active", tabName === "completion");

    if (tabName === "grades" && !gradesLoaded) {
        loadGrades();
    }

    if (tabName === "dropbox") {
        renderDropboxAssignments();
    }

    if (tabName === "discussion" && !discussionLoaded) {
        loadCourseDiscussionTopics();
    }

    if (tabName === "submissions") {
        renderCourseSubmissionsTab();
        loadQuizAnalyticsSummaries();
    }

    if (tabName === "completion") {
        loadCompletionRoster();
    }
}

function setGradeTabsVisible(visible) {
    const tabs = document.getElementById("course-tabs");
    const gradesTab = document.querySelector('.course-tab[data-course-tab="grades"]');
    const dropboxTab = document.querySelector('.course-tab[data-course-tab="dropbox"]');
    const discussionTab = document.getElementById("course-discussion-tab-btn");
    const submissionsTab = document.getElementById("course-submissions-tab-btn");
    const completionTab = document.getElementById("course-completion-tab-btn");

    if (tabs) {
        tabs.style.display = "flex";
    }

    if (gradesTab) {
        gradesTab.style.display = visible ? "inline-flex" : "none";
    }

    if (dropboxTab) {
        dropboxTab.style.display = visible ? "inline-flex" : "none";
    }

    if (discussionTab) {
        discussionTab.style.display = (isInstructor || isEnrolled) ? "inline-flex" : "none";
    }

    if (submissionsTab) {
        submissionsTab.style.display = isInstructor ? "inline-flex" : "none";
    }

    if (completionTab) {
        completionTab.style.display = isInstructor ? "inline-flex" : "none";
    }

    const activeTab = document.querySelector(".course-tab.active")?.dataset.courseTab;
    if (
        (!visible && ["grades", "dropbox"].includes(activeTab))
        || (!(isInstructor || isEnrolled) && activeTab === "discussion")
        || (visible && ["submissions", "completion"].includes(activeTab))
    ) {
        setActiveCourseTab("content");
    }
}

function completionStatusLabel(status) {
    if (status.manual_completed) return "Marked complete";
    if (status.automatic_completed) return "Automatically complete";
    return "In progress";
}

function completionSourceLabel(source) {
    if (source === "manual") return "Manual";
    if (source === "automatic") return "Automatic";
    return "None";
}

function completionBadgeClass(status) {
    if (status.completed) return "completion-badge complete";
    return "completion-badge pending";
}

function renderCompletionRoster() {
    const body = document.getElementById("completion-roster-body");
    const statusText = document.getElementById("completion-roster-status");

    if (!body) return;

    if (!completionRosterRows.length) {
        body.innerHTML = '<tr><td colspan="6" class="grades-empty">No enrolled learners found for this course.</td></tr>';
        if (statusText) statusText.textContent = "0 enrolled learners";
        return;
    }

    const completedCount = completionRosterRows.filter(row => row.status?.completed).length;
    if (statusText) {
        statusText.textContent = `${completedCount} of ${completionRosterRows.length} learners complete`;
    }

    body.innerHTML = completionRosterRows.map((row) => {
        const status = row.status || {};
        const progress = status.progress || {};
        const progressPercent = Math.max(0, Math.min(100, Number(progress.progress_percent || 0)));
        const checks = [
            ["Content", status.content_complete],
            ["Assignments", status.assignments_graded],
            ["Quizzes", status.quizzes_graded],
        ].map(([label, passed]) => `
            <span class="completion-check ${passed ? "passed" : "pending"}">
                ${escapeHtml(label)}
            </span>
        `).join("");
        const manualMeta = status.manual_completed_at
            ? `<span class="completion-meta">Marked ${escapeHtml(formatAssignmentDate(status.manual_completed_at))}</span>`
            : "";
        const note = status.manual_completion_note
            ? `<span class="completion-meta">${escapeHtml(status.manual_completion_note)}</span>`
            : "";
        const action = status.manual_completed
            ? `<button class="course-card-action-btn completion-undo-btn" type="button" data-completion-undo="${row.user_id}">Undo</button>`
            : `<button class="course-card-action-btn" type="button" data-completion-mark="${row.user_id}">Mark Complete</button>`;
        const certificate = row.certificate;
        const certificateCell = certificate?.verification_url
            ? `
                <div class="completion-certificate-actions">
                    <a class="course-card-action-btn" href="${escapeHtml(certificate.verification_url)}" target="_blank" rel="noopener">View</a>
                    <button class="course-card-action-btn" type="button" data-certificate-copy="${escapeHtml(certificate.verification_url)}">Copy</button>
                </div>
            `
            : `<span class="completion-meta">Available when complete</span>`;

        return `
            <tr>
                <td>
                    <strong>${escapeHtml(row.student_name || "Learner")}</strong>
                    <span class="completion-meta">${escapeHtml(row.student_email || "")}</span>
                </td>
                <td>
                    <div class="completion-progress">
                        <span>${progressPercent}%</span>
                        <div class="completion-progress-track" aria-hidden="true">
                            <div class="completion-progress-fill" style="width: ${progressPercent}%;"></div>
                        </div>
                    </div>
                </td>
                <td><div class="completion-checks">${checks}</div></td>
                <td>
                    <span class="${completionBadgeClass(status)}">${escapeHtml(completionStatusLabel(status))}</span>
                    <span class="completion-meta">Source: ${escapeHtml(completionSourceLabel(status.completion_source))}</span>
                    ${manualMeta}
                    ${note}
                </td>
                <td>${certificateCell}</td>
                <td class="completion-action-cell">${action}</td>
            </tr>
        `;
    }).join("");
}

async function loadCompletionRoster(force = false) {
    if (!isInstructor || (completionRosterLoaded && !force)) {
        renderCompletionRoster();
        return;
    }

    const body = document.getElementById("completion-roster-body");
    const statusText = document.getElementById("completion-roster-status");
    if (body) {
        body.innerHTML = '<tr><td colspan="6" class="grades-empty">Loading completion roster...</td></tr>';
    }
    if (statusText) {
        statusText.textContent = "";
    }

    try {
        const [rosterRows, certificateRows] = await Promise.all([
            axios.get(`/api/courses/${courseId}/completion-roster`)
                .then(response => Array.isArray(response.data) ? response.data : []),
            axios.get(`/api/courses/${courseId}/certificates`)
                .then(response => Array.isArray(response.data) ? response.data : []),
        ]);
        const certificatesByUser = new Map(certificateRows.map(row => [Number(row.user_id), row.certificate]));
        completionRosterRows = rosterRows.map(row => ({
            ...row,
            certificate: certificatesByUser.get(Number(row.user_id)) || null,
        }));
        completionRosterLoaded = true;
        renderCompletionRoster();
    } catch (error) {
        console.error("Failed to load completion roster:", error);
        if (body) {
            body.innerHTML = '<tr><td colspan="6" class="grades-empty text-danger">Unable to load completion roster.</td></tr>';
        }
    }
}

async function copyCertificateLink(url) {
    try {
        await navigator.clipboard.writeText(url);
        showActionMessage("Certificate link copied.", "success");
    } catch (_) {
        window.prompt("Certificate verification link:", url);
    }
}

async function markLearnerComplete(userId) {
    const row = completionRosterRows.find(item => Number(item.user_id) === Number(userId));
    const learnerName = row?.student_name || "this learner";
    const note = window.prompt(`Optional completion note for ${learnerName}:`, "");

    if (note === null) {
        return;
    }

    try {
        await axios.put(`/api/courses/${courseId}/completions/${userId}/manual`, {
            note: note.trim() || null,
        });
        completionRosterLoaded = false;
        await loadCompletionRoster(true);
    } catch (error) {
        showActionMessage(error.response?.data || "Unable to mark this learner complete.", "error");
    }
}

async function undoLearnerCompletion(userId) {
    const row = completionRosterRows.find(item => Number(item.user_id) === Number(userId));
    const learnerName = row?.student_name || "this learner";

    if (!window.confirm(`Undo manual completion for ${learnerName}?`)) {
        return;
    }

    try {
        await axios.delete(`/api/courses/${courseId}/completions/${userId}/manual`);
        completionRosterLoaded = false;
        await loadCompletionRoster(true);
    } catch (error) {
        showActionMessage(error.response?.data || "Unable to undo manual completion.", "error");
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

function formatGradeScore(score, maxScore, emptyLabel = "Pending") {
    const formattedScore = formatGradeNumber(score);
    const formattedMaxScore = formatGradeNumber(maxScore);

    if (formattedScore === null) {
        return emptyLabel;
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

function plainTextToHtml(value) {
    return escapeHtml(value).replace(/\n/g, "<br>");
}

function getDiscussionPreview(value, maxLength = 110) {
    const text = String(value ?? "").replace(/\s+/g, " ").trim();

    if (text.length <= maxLength) {
        return text;
    }

    return `${text.slice(0, maxLength).trim()}...`;
}

function formatDiscussionDate(value) {
    if (!value) {
        return "";
    }

    const date = new Date(value);
    if (Number.isNaN(date.getTime())) {
        return "";
    }

    return date.toLocaleString("en-SG", {
        month: "short",
        day: "numeric",
        year: "numeric",
        hour: "numeric",
        minute: "2-digit",
        timeZone: "Asia/Singapore",
    });
}

function discussionModuleTitle(moduleId) {
    const module = currentModules.find((item) => Number(item.module_id) === Number(moduleId));
    return module?.title || "Course discussion";
}

function showDiscussionStatus(message, type = "info") {
    const messageElement = document.getElementById("discussion-status-message");

    if (!messageElement) {
        return;
    }

    if (discussionStatusMessageTimer) {
        clearTimeout(discussionStatusMessageTimer);
    }

    messageElement.textContent = message;
    messageElement.className = message
        ? `course-action-message ${type} visible`
        : "course-action-message";

    if (message) {
        discussionStatusMessageTimer = setTimeout(() => {
            messageElement.classList.remove("visible");
        }, 4500);
    }
}

function renderDiscussionBreadcrumbs(items = []) {
    const breadcrumbs = document.getElementById("discussion-breadcrumbs");

    if (!breadcrumbs) {
        return;
    }

    breadcrumbs.innerHTML = items.map((item, index) => {
        const isLast = index === items.length - 1;
        return isLast
            ? `<span>${escapeHtml(item.label)}</span>`
            : `<button type="button" data-discussion-crumb="${escapeHtml(item.view)}">${escapeHtml(item.label)}</button><i class="bi bi-chevron-right" aria-hidden="true"></i>`;
    }).join("");
}

function setDiscussionView(view) {
    document.getElementById("discussion-topic-list").hidden = view !== "topics";
    document.getElementById("discussion-thread-list").hidden = view !== "threads";
    document.getElementById("discussion-thread-detail").hidden = view !== "detail";

    const backButton = document.getElementById("discussion-back-btn");
    if (backButton) {
        backButton.hidden = view === "topics";
        backButton.dataset.discussionBack = view === "detail" ? "threads" : "topics";
        const backLabel = backButton.querySelector("span");
        if (backLabel) {
            backLabel.textContent = view === "detail"
                ? "Back to Topic"
                : "Back to Discussions";
        }
    }

    const addTopicButton = document.getElementById("add-discussion-topic-btn");
    if (addTopicButton) {
        addTopicButton.style.display = view === "topics" && isInstructor ? "inline-flex" : "none";
    }
}

function renderDiscussionPagination(meta, target) {
    if (!meta || Number(meta.total_pages || 1) <= 1) {
        return "";
    }

    const page = Number(meta.page || 1);
    const totalPages = Number(meta.total_pages || 1);
    const total = Number(meta.total || 0);
    const pageSize = Number(meta.page_size || 10);
    const start = total === 0 ? 0 : ((page - 1) * pageSize) + 1;
    const end = Math.min(page * pageSize, total);

    return `
        <div class="discussion-pagination">
            <span>${start}-${end} of ${total}</span>
            <div>
                <button type="button" data-discussion-page-target="${target}" data-discussion-page="${page - 1}" ${page <= 1 ? "disabled" : ""}>Previous</button>
                <span>Page ${page} of ${totalPages}</span>
                <button type="button" data-discussion-page-target="${target}" data-discussion-page="${page + 1}" ${page >= totalPages ? "disabled" : ""}>Next</button>
            </div>
        </div>
    `;
}

async function loadCourseDiscussionTopics(page = currentDiscussionTopicPage) {
    const list = document.getElementById("discussion-topic-list");

    if (!list) {
        return;
    }

    setDiscussionView("topics");
    currentDiscussionThreadPagination = null;
    currentDiscussionThreadPage = 1;
    renderDiscussionBreadcrumbs([{ label: "Discussions", view: "topics" }]);

    try {
        const response = await axios.get(`/api/discussions/courses/${courseId}/topics`, {
            params: { page, page_size: DISCUSSION_TOPIC_PAGE_SIZE },
        });
        currentDiscussionTopics = Array.isArray(response.data?.items) ? response.data.items : [];
        currentDiscussionTopicPagination = response.data;
        currentDiscussionTopicPage = Number(response.data?.page || page || 1);
        discussionLoaded = true;

        if (!currentDiscussionTopics.length) {
            if (currentDiscussionTopicPage > 1 && Number(currentDiscussionTopicPagination?.total || 0) > 0) {
                await loadCourseDiscussionTopics(currentDiscussionTopicPage - 1);
                return;
            }

            list.innerHTML = `
                <div class="discussion-empty">
                    <i class="bi bi-chat-square-text" aria-hidden="true"></i>
                    <p>No discussion topics yet.</p>
                </div>
            `;
            return;
        }

        list.innerHTML = `
            <div class="discussion-table">
                <div class="discussion-table-head">
                    <span>Topic</span>
                    <span>Threads</span>
                    <span>Posts</span>
                    <span>Created By</span>
                    <span></span>
                </div>
                ${currentDiscussionTopics.map((topic) => {
                    const topicActionMenu = topic.can_manage
                        ? `
                            <details class="discussion-actions-menu discussion-row-actions">
                                <summary aria-label="Topic actions">
                                    <i class="bi bi-chevron-down" aria-hidden="true"></i>
                                </summary>
                                <div class="discussion-actions-dropdown">
                                    <button type="button" data-topic-edit="${topic.topic_id}"><i class="bi bi-pencil" aria-hidden="true"></i><span>Edit Topic</span></button>
                                    <button type="button" data-topic-lock="${topic.topic_id}" data-topic-lock-value="${topic.is_locked ? "false" : "true"}">
                                        <i class="bi ${topic.is_locked ? "bi-unlock" : "bi-lock"}" aria-hidden="true"></i>
                                        <span>${topic.is_locked ? "Unlock Topic" : "Lock Topic"}</span>
                                    </button>
                                    ${topic.can_delete ? `<button class="danger" type="button" data-topic-delete="${topic.topic_id}">
                                        <i class="bi bi-trash" aria-hidden="true"></i>
                                        <span>Delete Topic</span>
                                    </button>` : ""}
                                </div>
                            </details>
                        `
                        : "";

                    return `
                        <article class="discussion-topic-row">
                            <button class="discussion-topic-main" type="button" data-topic-open="${topic.topic_id}">
                                <span>
                                    <strong>${escapeHtml(topic.title)}</strong>
                                    <small>${escapeHtml(discussionModuleTitle(topic.module_id))}</small>
                                    ${topic.description ? `<small>${escapeHtml(topic.description).slice(0, 140)}</small>` : ""}
                                </span>
                                <span>${topic.thread_count}</span>
                                <span>${topic.post_count}</span>
                                <span>
                                    <strong>${escapeHtml(topic.author?.name || "Unknown user")}</strong>
                                    <small>${formatDiscussionDate(topic.created_at)}</small>
                                </span>
                            </button>
                            ${topicActionMenu}
                        </article>
                    `;
                }).join("")}
            </div>
            ${renderDiscussionPagination(currentDiscussionTopicPagination, "topics")}
        `;
    } catch (error) {
        list.innerHTML = '<p class="grades-error">Unable to load course discussions.</p>';
        console.error("Failed to load course discussions:", error);
    }
}

function renderDiscussionThreadItems(threads) {
    if (!threads.length) {
        return '<div class="discussion-empty compact"><p>No threads yet.</p></div>';
    }

    return threads.map((thread) => {
        const threadActionMenu = thread.can_edit || thread.can_close || thread.can_hide
            ? `
                <details class="discussion-actions-menu discussion-row-actions">
                    <summary aria-label="Thread actions">
                        <i class="bi bi-chevron-down" aria-hidden="true"></i>
                    </summary>
                    <div class="discussion-actions-dropdown">
                        ${thread.can_edit ? `<button type="button" data-thread-edit="${thread.thread_id}"><i class="bi bi-pencil" aria-hidden="true"></i><span>Edit</span></button>` : ""}
                        ${thread.can_close ? `<button class="danger" type="button" data-thread-close="${thread.thread_id}"><i class="bi bi-lock" aria-hidden="true"></i><span>Close</span></button>` : ""}
                        ${thread.can_hide ? `<button class="danger" type="button" data-thread-hide="${thread.thread_id}"><i class="bi bi-eye-slash" aria-hidden="true"></i><span>Remove</span></button>` : ""}
                    </div>
                </details>
            `
            : "";

        return `
        <article class="discussion-thread-row">
        <button class="discussion-thread-main" type="button" data-thread-open="${thread.thread_id}">
            <span>
                <strong>${escapeHtml(thread.title)}</strong>
                <small>${escapeHtml(thread.author.name)} · posted ${formatDiscussionDate(thread.created_at)}</small>
                <span class="discussion-thread-preview">${escapeHtml(thread.body).slice(0, 180)}</span>
            </span>
            <span>${thread.status === "closed" ? '<span class="discussion-status-pill closed">Closed</span>' : '<span class="discussion-status-pill">Open</span>'}</span>
            <span>${thread.reply_count}<small>Replies</small></span>
            <span>${thread.view_count}<small>Views</small></span>
        </button>
        ${threadActionMenu}
        </article>
    `;
    }).join("");
}

function splitAuthorName(name) {
    const parts = String(name || "").trim().split(/\s+/).filter(Boolean);

    return {
        first: parts[0] || "",
        last: parts.length > 1 ? parts[parts.length - 1] : parts[0] || "",
    };
}

function compareText(first, second) {
    return String(first || "").localeCompare(String(second || ""), undefined, {
        sensitivity: "base",
        numeric: true,
    });
}

function sortedDiscussionThreads(mode = "threaded") {
    const threads = [...currentDiscussionThreads];

    switch (mode) {
        case "newest":
            return threads.sort((first, second) => new Date(second.created_at) - new Date(first.created_at));
        case "oldest":
            return threads.sort((first, second) => new Date(first.created_at) - new Date(second.created_at));
        case "author-first-az":
            return threads.sort((first, second) => compareText(splitAuthorName(first.author?.name).first, splitAuthorName(second.author?.name).first));
        case "author-first-za":
            return threads.sort((first, second) => compareText(splitAuthorName(second.author?.name).first, splitAuthorName(first.author?.name).first));
        case "author-last-az":
            return threads.sort((first, second) => compareText(splitAuthorName(first.author?.name).last, splitAuthorName(second.author?.name).last));
        case "author-last-za":
            return threads.sort((first, second) => compareText(splitAuthorName(second.author?.name).last, splitAuthorName(first.author?.name).last));
        case "subject-az":
            return threads.sort((first, second) => compareText(first.title, second.title));
        case "subject-za":
            return threads.sort((first, second) => compareText(second.title, first.title));
        case "threaded":
        default:
            return threads.sort((first, second) => new Date(second.updated_at) - new Date(first.updated_at));
    }
}

function applyDiscussionThreadShowMode() {
    const select = document.getElementById("discussion-thread-show");
    const items = document.getElementById("discussion-thread-items");

    if (!items) {
        return;
    }

    items.innerHTML = renderDiscussionThreadItems(sortedDiscussionThreads(select?.value || "threaded"));
}

async function openDiscussionTopic(topicId, page = currentDiscussionThreadPage) {
    currentDiscussionTopic = currentDiscussionTopics.find((topic) => topic.topic_id === Number(topicId)) || null;
    const list = document.getElementById("discussion-thread-list");

    if (!currentDiscussionTopic || !list) {
        return;
    }

    setDiscussionView("threads");
    renderDiscussionBreadcrumbs([
        { label: "Discussions List", view: "topics" },
        { label: "View Topic", view: "threads" },
    ]);

    try {
        const response = await axios.get(`/api/discussions/topics/${topicId}/threads`, {
            params: { page, page_size: DISCUSSION_THREAD_PAGE_SIZE },
        });
        currentDiscussionThreads = Array.isArray(response.data?.items) ? response.data.items : [];
        currentDiscussionThreadPagination = response.data;
        currentDiscussionThreadPage = Number(response.data?.page || page || 1);
        if (!currentDiscussionThreads.length && currentDiscussionThreadPage > 1 && Number(currentDiscussionThreadPagination?.total || 0) > 0) {
            await openDiscussionTopic(topicId, currentDiscussionThreadPage - 1);
            return;
        }
        const canCreateThread = Boolean(currentDiscussionTopic.can_create_thread);
        const manageActions = currentDiscussionTopic.can_manage
            ? `<button class="discussion-link-btn" type="button" data-topic-edit="${currentDiscussionTopic.topic_id}"><i class="bi bi-pencil" aria-hidden="true"></i> Edit Topic</button>`
            : "";

        list.innerHTML = `
            <div class="discussion-topic-hero">
                <div class="discussion-topic-title-row">
                    <h2>${escapeHtml(currentDiscussionTopic.title)}</h2>
                    ${currentDiscussionTopic.is_locked ? '<span class="discussion-status-pill closed">Locked</span>' : ""}
                </div>
                <p class="discussion-module-label">${escapeHtml(discussionModuleTitle(currentDiscussionTopic.module_id))}</p>
                <div class="discussion-topic-body">${plainTextToHtml(currentDiscussionTopic.description || "")}</div>
                <div class="discussion-topic-actions">
                    ${canCreateThread ? '<button id="start-thread-btn" class="course-card-action-btn" type="button">Start a New Thread</button>' : ""}
                    ${manageActions}
                </div>
            </div>
            <div class="discussion-thread-board">
                <div class="discussion-thread-controls">
                    <label>Show:
                        <select id="discussion-thread-show">
                            <option value="threaded">Threaded</option>
                            <option value="newest">Newest First</option>
                            <option value="oldest">Oldest First</option>
                            <option value="author-first-az">Author First Name A-Z</option>
                            <option value="author-first-za">Author First Name Z-A</option>
                            <option value="author-last-az">Author Last Name A-Z</option>
                            <option value="author-last-za">Author Last Name Z-A</option>
                            <option value="subject-az">Subject A-Z</option>
                            <option value="subject-za">Subject Z-A</option>
                        </select>
                    </label>
                </div>
                <div id="discussion-thread-items">
                    ${renderDiscussionThreadItems(sortedDiscussionThreads("threaded"))}
                </div>
            </div>
            ${renderDiscussionPagination(currentDiscussionThreadPagination, "threads")}
        `;
    } catch (error) {
        list.innerHTML = '<p class="grades-error">Unable to load this discussion topic.</p>';
        console.error("Failed to load discussion topic:", error);
    }
}

async function openDiscussionThread(threadId, page = currentDiscussionReplyPage) {
    const detail = document.getElementById("discussion-thread-detail");

    if (!detail) {
        return;
    }

    setDiscussionView("detail");
    renderDiscussionBreadcrumbs([
        { label: "Discussions List", view: "topics" },
        { label: currentDiscussionTopic?.title || "Topic", view: "threads" },
        { label: "Thread", view: "detail" },
    ]);

    try {
        const response = await axios.get(`/api/discussions/threads/${threadId}`, {
            params: { page, page_size: DISCUSSION_REPLY_PAGE_SIZE },
        });
        currentDiscussionDetail = response.data;
        currentDiscussionReplyPage = Number(response.data?.replies_page || page || 1);
        currentDiscussionReplyPagination = {
            page: currentDiscussionReplyPage,
            page_size: Number(response.data?.replies_page_size || DISCUSSION_REPLY_PAGE_SIZE),
            total: Number(response.data?.replies_total || 0),
            total_pages: Number(response.data?.replies_total_pages || 1),
        };
        if (!currentDiscussionDetail.replies?.length && currentDiscussionReplyPage > 1 && currentDiscussionReplyPagination.total > 0) {
            await openDiscussionThread(threadId, currentDiscussionReplyPage - 1);
            return;
        }
        renderDiscussionThreadDetail();
    } catch (error) {
        detail.innerHTML = '<p class="grades-error">Unable to load this thread.</p>';
        console.error("Failed to load discussion thread:", error);
    }
}

function renderDiscussionThreadDetail() {
    const detail = document.getElementById("discussion-thread-detail");
    const thread = currentDiscussionDetail?.thread;
    const replies = currentDiscussionDetail?.replies || [];

    if (!detail || !thread) {
        return;
    }

    const threadActionMenu = thread.can_edit || thread.can_close || thread.can_hide
        ? `
            <details class="discussion-actions-menu">
                <summary aria-label="Thread actions">
                    <i class="bi bi-chevron-down" aria-hidden="true"></i>
                </summary>
                <div class="discussion-actions-dropdown">
                    ${thread.can_edit ? `<button type="button" data-thread-edit="${thread.thread_id}"><i class="bi bi-pencil" aria-hidden="true"></i><span>Edit</span></button>` : ""}
                    ${thread.can_close ? `<button class="danger" type="button" data-thread-close="${thread.thread_id}"><i class="bi bi-lock" aria-hidden="true"></i><span>Close</span></button>` : ""}
                    ${thread.can_hide ? `<button class="danger" type="button" data-thread-hide="${thread.thread_id}"><i class="bi bi-eye-slash" aria-hidden="true"></i><span>Remove</span></button>` : ""}
                </div>
            </details>
        `
        : "";

    const repliesByParent = replies.reduce((groups, reply) => {
        const parentKey = reply.parent_reply_id || 0;
        if (!groups.has(parentKey)) {
            groups.set(parentKey, []);
        }
        groups.get(parentKey).push(reply);
        return groups;
    }, new Map());
    const repliesById = new Map(replies.map((reply) => [reply.reply_id, reply]));
    const renderReplies = (parentReplyId = 0, depth = 0) => (repliesByParent.get(parentReplyId) || [])
        .map((reply) => {
            const parentReply = reply.parent_reply_id ? repliesById.get(reply.parent_reply_id) : null;
            const parentContext = parentReply ? `
                <div class="discussion-reply-context">
                    <span>Replying to ${escapeHtml(parentReply.author.name)}</span>
                    <p>${escapeHtml(getDiscussionPreview(parentReply.body))}</p>
                </div>
            ` : "";
            const replyActions = thread.can_reply || reply.can_delete
                ? `
                    <div class="discussion-reply-actions">
                        ${thread.can_reply ? `<button class="discussion-link-btn" type="button" data-reply-to="${reply.reply_id}" data-reply-author="${escapeHtml(reply.author.name)}">Reply</button>` : ""}
                        ${reply.can_delete ? `<button class="discussion-link-btn danger" type="button" data-reply-delete="${reply.reply_id}">Delete</button>` : ""}
                    </div>
                `
                : "";

            return `
            <article class="discussion-reply ${depth > 0 ? "child" : ""}" style="--reply-depth: ${Math.min(depth, 4)};">
                <div class="discussion-avatar"><i class="bi bi-person" aria-hidden="true"></i></div>
                <div>
                    ${parentContext}
                    <div class="discussion-reply-head">
                        <strong>${escapeHtml(reply.author.name)}</strong>
                        <span>${formatDiscussionDate(reply.created_at)}</span>
                    </div>
                    <div class="discussion-reply-body">${plainTextToHtml(reply.body)}</div>
                    ${replyActions}
                </div>
            </article>
            ${renderReplies(reply.reply_id, depth + 1)}
        `;
        })
        .join("");

    detail.innerHTML = `
        <article class="discussion-post">
            <div class="discussion-post-head">
                <div>
                    <h2>${escapeHtml(thread.title)}</h2>
                    <p>${escapeHtml(thread.author.name)} · posted ${formatDiscussionDate(thread.created_at)}</p>
                </div>
                <div class="discussion-post-actions">
                    ${thread.status === "closed" ? '<span class="discussion-status-pill closed">Closed</span>' : '<span class="discussion-status-pill">Open</span>'}
                    ${threadActionMenu}
                </div>
            </div>
            <div class="discussion-post-body">${plainTextToHtml(thread.body)}</div>
            <div class="discussion-post-meta">${thread.reply_count} replies · ${thread.view_count} views</div>
        </article>
        <div class="discussion-replies">
            ${renderReplies()}
        </div>
        ${renderDiscussionPagination(currentDiscussionReplyPagination, "replies")}
        ${thread.can_reply ? `
            <div class="discussion-reply-box">
                <div class="discussion-reply-box-head">
                    <label for="discussion-new-reply-input">Reply</label>
                    <button id="clear-reply-target-btn" class="discussion-link-btn" type="button" hidden>Reply to thread</button>
                </div>
                <p id="discussion-reply-target" class="discussion-reply-target" hidden></p>
                <textarea id="discussion-new-reply-input" rows="4" placeholder="Write a reply"></textarea>
                <button id="post-discussion-reply-btn" class="course-card-action-btn" type="button">Post Reply</button>
            </div>
        ` : ""}
    `;
    currentReplyParentId = null;
}

function getGradeDateLabel(value, prefix) {
    if (!value) {
        return "";
    }

    return `${prefix}: ${formatAssignmentDate(value)}`;
}

function buildGradeRow({
    title,
    meta,
    score,
    maxScore,
    resultLabel = "",
    feedback,
    actionHtml = "",
    emptyScoreLabel = "Pending",
    emptyScoreClass = "pending",
}) {
    const hasScore = score !== null && score !== undefined;
    const percent = getGradePercent(score, maxScore);
    const percentLabel = percent === null ? "" : ` (${percent}%)`;

    return `
        <div class="grade-row">
            <div>
                <div class="grade-title">${escapeHtml(title)}</div>
                <div class="grade-meta">${escapeHtml(meta || "No activity yet")}</div>
            </div>
            <div class="grade-score ${hasScore ? "" : emptyScoreClass}">
                ${formatGradeScore(score, maxScore, emptyScoreLabel)}${percentLabel}
                ${resultLabel}
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
        const hasSubmission = Boolean(assignment.submitted_at);
        const metaParts = [
            getGradeDateLabel(assignment.submitted_at, "Submitted"),
            !hasSubmission ? getGradeDateLabel(assignment.due_date, "Due") : "",
        ].filter(Boolean);

        return buildGradeRow({
            title: assignment.title || "Untitled assignment",
            meta: metaParts.join(" - "),
            score: assignment.score,
            maxScore: assignment.max_score,
            feedback: assignment.feedback,
            emptyScoreLabel: hasSubmission ? "Pending" : "Not submitted",
            emptyScoreClass: hasSubmission ? "pending" : "not-submitted",
        });
    });

    const quizRows = quizzes.map((quiz) => {
        const canViewAnswers = quiz.is_graded && quiz.attempt_id;
        const resultLabel = quiz.is_graded && quiz.passed !== null && quiz.passed !== undefined
            ? `<span class="${quiz.passed ? "quiz-pass-status pass" : "quiz-pass-status fail"}">${quiz.passed ? "Pass" : "Fail"}</span>`
            : "";

        return buildGradeRow({
            title: quiz.title || "Untitled quiz",
            meta: [
                getGradeDateLabel(quiz.submitted_at, "Submitted") || (quiz.attempt_id ? "Attempt in progress" : ""),
                `Passing mark: ${quiz.passing_mark ?? 50}%`,
            ].filter(Boolean).join(" - "),
            score: quiz.total_score,
            maxScore: quiz.max_score,
            resultLabel,
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

function populateDiscussionModuleOptions(selectedModuleId = null) {
    const moduleInput = document.getElementById("discussion-topic-module-input");

    if (!moduleInput) {
        return;
    }

    moduleInput.innerHTML = currentModules
        .map((module) => {
            const moduleId = Number(module.module_id);
            const selected = Number(selectedModuleId) === moduleId ? "selected" : "";
            return `<option value="${moduleId}" ${selected}>${escapeHtml(module.position)}. ${escapeHtml(module.title || "Untitled module")}</option>`;
        })
        .join("");
}

function openDiscussionTopicModal(topic = null) {
    currentEditingTopicId = topic?.topic_id || null;
    populateDiscussionModuleOptions(topic?.module_id || currentModules[0]?.module_id);
    document.getElementById("discussion-topic-modal-title").textContent = topic ? "Edit Discussion Topic" : "New Discussion Topic";
    document.getElementById("discussion-topic-module-input").disabled = Boolean(topic);
    document.getElementById("discussion-topic-title-input").value = topic?.title || "";
    document.getElementById("discussion-topic-description-input").value = topic?.description || "";
    document.getElementById("discussion-topic-locked-input").checked = Boolean(topic?.is_locked);
    document.getElementById("discussion-topic-modal").style.display = "flex";
}

function closeDiscussionTopicModal() {
    currentEditingTopicId = null;
    document.getElementById("discussion-topic-title-input").value = "";
    document.getElementById("discussion-topic-description-input").value = "";
    document.getElementById("discussion-topic-locked-input").checked = false;
    document.getElementById("discussion-topic-modal").style.display = "none";
}

async function saveDiscussionTopic() {
    const moduleId = Number(document.getElementById("discussion-topic-module-input").value || 0);
    const title = document.getElementById("discussion-topic-title-input").value.trim();
    const description = document.getElementById("discussion-topic-description-input").value.trim();
    const isLocked = document.getElementById("discussion-topic-locked-input").checked;

    if (!moduleId) {
        alert("Please create a module before adding a discussion topic.");
        return;
    }

    if (!title) {
        alert("Please enter a topic title");
        return;
    }

    try {
        if (currentEditingTopicId) {
            await axios.put(`/api/discussions/topics/${currentEditingTopicId}`, {
                title,
                description,
                is_locked: isLocked,
            });
        } else {
            await axios.post("/api/discussions/topics", {
                module_id: moduleId,
                title,
                description,
            });
        }

        closeDiscussionTopicModal();
        discussionLoaded = false;
        await loadCourseDiscussionTopics(currentDiscussionTopicPage);
        showDiscussionStatus("Discussion topic saved.", "success");
    } catch (error) {
        showDiscussionStatus(error.response?.data || "Failed to save discussion topic.", "error");
    }
}

async function setDiscussionTopicLocked(topicId, isLocked) {
    const topic = currentDiscussionTopics.find((item) => Number(item.topic_id) === Number(topicId));

    if (!topic) {
        return;
    }

    try {
        await axios.put(`/api/discussions/topics/${topicId}`, {
            title: topic.title,
            description: topic.description || "",
            is_locked: isLocked,
        });
        discussionLoaded = false;
        await loadCourseDiscussionTopics(currentDiscussionTopicPage);
        showDiscussionStatus(isLocked ? "Topic locked." : "Topic unlocked.", "success");
    } catch (error) {
        showDiscussionStatus(error.response?.data || "Failed to update topic.", "error");
    }
}

async function deleteDiscussionTopic(topicId) {
    if (!confirm("Delete this topic and all threads and replies under it?")) {
        return;
    }

    try {
        await axios.delete(`/api/discussions/topics/${topicId}`);
        discussionLoaded = false;
        currentDiscussionTopic = null;
        await loadCourseDiscussionTopics(currentDiscussionTopicPage);
        showDiscussionStatus("Topic deleted.", "success");
    } catch (error) {
        showDiscussionStatus(error.response?.data || "Failed to delete topic.", "error");
    }
}

function openDiscussionThreadModal(thread = null) {
    currentEditingThreadId = thread?.thread_id || null;
    document.getElementById("discussion-thread-modal-title").textContent = thread ? "Edit Thread" : "Start a New Thread";
    document.getElementById("discussion-thread-title-input").value = thread?.title || "";
    document.getElementById("discussion-thread-body-input").value = thread?.body || "";
    document.getElementById("discussion-thread-modal").style.display = "flex";
}

function closeDiscussionThreadModal() {
    currentEditingThreadId = null;
    document.getElementById("discussion-thread-title-input").value = "";
    document.getElementById("discussion-thread-body-input").value = "";
    document.getElementById("discussion-thread-modal").style.display = "none";
}

async function saveDiscussionThread() {
    const title = document.getElementById("discussion-thread-title-input").value.trim();
    const body = document.getElementById("discussion-thread-body-input").value.trim();

    if (!title || !body) {
        alert("Please enter a title and post");
        return;
    }

    try {
        if (currentEditingThreadId) {
            await axios.put(`/api/discussions/threads/${currentEditingThreadId}`, { title, body });
            closeDiscussionThreadModal();
            await openDiscussionThread(currentEditingThreadId, currentDiscussionReplyPage);
        } else {
            await axios.post(`/api/discussions/topics/${currentDiscussionTopic.topic_id}/threads`, { title, body });
            closeDiscussionThreadModal();
            await openDiscussionTopic(currentDiscussionTopic.topic_id, currentDiscussionThreadPage);
        }
        discussionLoaded = false;
        showDiscussionStatus("Thread saved.", "success");
    } catch (error) {
        showDiscussionStatus(error.response?.data || "Failed to save thread.", "error");
    }
}

async function postDiscussionReply() {
    const input = document.getElementById("discussion-new-reply-input");
    const body = input?.value.trim();

    if (!body) {
        alert("Please enter a reply");
        return;
    }

    try {
        await axios.post(`/api/discussions/threads/${currentDiscussionDetail.thread.thread_id}/replies`, {
            body,
            parent_reply_id: currentReplyParentId,
        });
        await openDiscussionThread(currentDiscussionDetail.thread.thread_id, currentDiscussionReplyPage);
        discussionLoaded = false;
        showDiscussionStatus("Reply posted.", "success");
    } catch (error) {
        showDiscussionStatus(error.response?.data || "Failed to post reply.", "error");
    }
}

function setReplyTarget(replyId, authorName) {
    currentReplyParentId = Number(replyId);
    const target = document.getElementById("discussion-reply-target");
    const clearButton = document.getElementById("clear-reply-target-btn");
    const input = document.getElementById("discussion-new-reply-input");

    if (target) {
        target.textContent = `Replying to ${authorName}`;
        target.hidden = false;
    }
    if (clearButton) {
        clearButton.hidden = false;
    }
    input?.focus();
}

function clearReplyTarget() {
    currentReplyParentId = null;
    const target = document.getElementById("discussion-reply-target");
    const clearButton = document.getElementById("clear-reply-target-btn");

    if (target) {
        target.hidden = true;
        target.textContent = "";
    }
    if (clearButton) {
        clearButton.hidden = true;
    }
}

async function closeDiscussionThread(threadId, returnToTopic = false) {
    if (!confirm("Close this thread?")) {
        return;
    }

    try {
        await axios.post(`/api/discussions/threads/${threadId}/close`);
        discussionLoaded = false;
        if (returnToTopic && currentDiscussionTopic) {
            await openDiscussionTopic(currentDiscussionTopic.topic_id, currentDiscussionThreadPage);
        } else {
            await openDiscussionThread(threadId, currentDiscussionReplyPage);
        }
        showDiscussionStatus("Thread closed.", "success");
    } catch (error) {
        showDiscussionStatus(error.response?.data || "Failed to close thread.", "error");
    }
}

async function hideDiscussionThread(threadId) {
    if (!confirm("Remove this thread from discussions?")) {
        return;
    }

    try {
        await axios.post(`/api/discussions/threads/${threadId}/hide`);
        discussionLoaded = false;
        showDiscussionStatus("Thread removed.", "success");
        if (currentDiscussionTopic) {
            await openDiscussionTopic(currentDiscussionTopic.topic_id, currentDiscussionThreadPage);
        } else {
            await loadCourseDiscussionTopics(currentDiscussionTopicPage);
        }
    } catch (error) {
        showDiscussionStatus(error.response?.data || "Failed to remove thread.", "error");
    }
}

async function deleteDiscussionReply(replyId) {
    if (!confirm("Delete this reply and all replies under it?")) {
        return;
    }

    try {
        await axios.delete(`/api/discussions/replies/${replyId}`);
        await openDiscussionThread(currentDiscussionDetail.thread.thread_id, currentDiscussionReplyPage);
        discussionLoaded = false;
        showDiscussionStatus("Reply deleted.", "success");
    } catch (error) {
        showDiscussionStatus(error.response?.data || "Failed to delete reply.", "error");
    }
}

function bindDiscussionControls() {
    document.getElementById("add-discussion-topic-btn")?.addEventListener("click", () => openDiscussionTopicModal());
    document.getElementById("save-discussion-topic-btn")?.addEventListener("click", saveDiscussionTopic);
    document.getElementById("close-discussion-topic-modal-btn")?.addEventListener("click", closeDiscussionTopicModal);
    document.getElementById("save-discussion-thread-btn")?.addEventListener("click", saveDiscussionThread);
    document.getElementById("close-discussion-thread-modal-btn")?.addEventListener("click", closeDiscussionThreadModal);
    document.getElementById("course-discussion-panel")?.addEventListener("click", (event) => {
        const topicOpen = event.target.closest("[data-topic-open]");
        const threadOpen = event.target.closest("[data-thread-open]");
        const crumb = event.target.closest("[data-discussion-crumb]");
        const topicEdit = event.target.closest("[data-topic-edit]");
        const topicLock = event.target.closest("[data-topic-lock]");
        const topicDelete = event.target.closest("[data-topic-delete]");
        const threadEdit = event.target.closest("[data-thread-edit]");
        const threadClose = event.target.closest("[data-thread-close]");
        const threadHide = event.target.closest("[data-thread-hide]");
        const backButton = event.target.closest("#discussion-back-btn");
        const replyTo = event.target.closest("[data-reply-to]");
        const replyDelete = event.target.closest("[data-reply-delete]");
        const clearReplyTargetButton = event.target.closest("#clear-reply-target-btn");
        const startThread = event.target.closest("#start-thread-btn");
        const postReply = event.target.closest("#post-discussion-reply-btn");
        const pageButton = event.target.closest("[data-discussion-page-target]");

        if (topicEdit) {
            const topic = currentDiscussionTopics.find((item) => Number(item.topic_id) === Number(topicEdit.dataset.topicEdit)) || currentDiscussionTopic;
            openDiscussionTopicModal(topic);
        } else if (topicLock) {
            setDiscussionTopicLocked(topicLock.dataset.topicLock, topicLock.dataset.topicLockValue === "true");
        } else if (topicDelete) {
            deleteDiscussionTopic(topicDelete.dataset.topicDelete);
        } else if (topicOpen) {
            currentDiscussionThreadPage = 1;
            openDiscussionTopic(topicOpen.dataset.topicOpen, 1);
        } else if (threadEdit) {
            openDiscussionThreadModal(currentDiscussionThreads.find((thread) => Number(thread.thread_id) === Number(threadEdit.dataset.threadEdit)) || currentDiscussionDetail?.thread);
        } else if (threadClose) {
            closeDiscussionThread(threadClose.dataset.threadClose, Boolean(threadClose.closest(".discussion-thread-row")));
        } else if (threadHide) {
            hideDiscussionThread(threadHide.dataset.threadHide);
        } else if (threadOpen) {
            currentDiscussionReplyPage = 1;
            openDiscussionThread(threadOpen.dataset.threadOpen, 1);
        } else if (crumb?.dataset.discussionCrumb === "topics") {
            loadCourseDiscussionTopics(currentDiscussionTopicPage);
        } else if (crumb?.dataset.discussionCrumb === "threads" && currentDiscussionTopic) {
            openDiscussionTopic(currentDiscussionTopic.topic_id, currentDiscussionThreadPage);
        } else if (backButton?.dataset.discussionBack === "threads" && currentDiscussionTopic) {
            openDiscussionTopic(currentDiscussionTopic.topic_id, currentDiscussionThreadPage);
        } else if (backButton?.dataset.discussionBack === "topics") {
            loadCourseDiscussionTopics(currentDiscussionTopicPage);
        } else if (pageButton?.dataset.discussionPageTarget === "topics") {
            loadCourseDiscussionTopics(Number(pageButton.dataset.discussionPage || 1));
        } else if (pageButton?.dataset.discussionPageTarget === "threads" && currentDiscussionTopic) {
            openDiscussionTopic(currentDiscussionTopic.topic_id, Number(pageButton.dataset.discussionPage || 1));
        } else if (pageButton?.dataset.discussionPageTarget === "replies" && currentDiscussionDetail?.thread) {
            openDiscussionThread(currentDiscussionDetail.thread.thread_id, Number(pageButton.dataset.discussionPage || 1));
        } else if (replyTo) {
            setReplyTarget(replyTo.dataset.replyTo, replyTo.dataset.replyAuthor || "this reply");
        } else if (replyDelete) {
            deleteDiscussionReply(replyDelete.dataset.replyDelete);
        } else if (clearReplyTargetButton) {
            clearReplyTarget();
        } else if (startThread) {
            openDiscussionThreadModal();
        } else if (postReply) {
            postDiscussionReply();
        }
    });

    document.getElementById("course-discussion-panel")?.addEventListener("change", (event) => {
        if (event.target.closest("#discussion-thread-show")) {
            applyDiscussionThreadShowMode();
        }
    });
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
    document.getElementById("confirm-delete-quiz-attempt-btn")?.addEventListener("click", confirmDeleteQuizAttempt);
    document.getElementById("cancel-delete-quiz-attempt-btn")?.addEventListener("click", closeDeleteQuizAttemptModal);
    document.getElementById("close-quiz-analytics-btn")?.addEventListener("click", closeQuizAnalytics);
    document.getElementById("submit-assignment-dropbox-btn")?.addEventListener("click", submitAssignmentDropbox);
    document.getElementById("refresh-grades-btn")?.addEventListener("click", () => {
        gradesLoaded = false;
        loadGrades();
    });
    document.getElementById("refresh-completion-roster-btn")?.addEventListener("click", () => {
        completionRosterLoaded = false;
        loadCompletionRoster(true);
    });
    document.getElementById("completion-roster-body")?.addEventListener("click", (event) => {
        const markButton = event.target.closest("[data-completion-mark]");
        const undoButton = event.target.closest("[data-completion-undo]");
        const copyButton = event.target.closest("[data-certificate-copy]");

        if (markButton) {
            markLearnerComplete(markButton.dataset.completionMark);
        } else if (undoButton) {
            undoLearnerCompletion(undoButton.dataset.completionUndo);
        } else if (copyButton) {
            copyCertificateLink(copyButton.dataset.certificateCopy || "");
        }
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

    bindDiscussionControls();
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
