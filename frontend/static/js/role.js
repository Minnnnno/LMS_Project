// role.js — Role display helpers (depends on lms-core.js)

window.RoleUtils = {
    badge(roleName) {
        const cls = roleName.includes("Admin")      ? "badge-admin"
                  : roleName.includes("Instructor") ? "badge-instructor"
                  : "badge-student";
        return `<span class="badge-role ${cls}">${HtmlUtils.escape(roleName)}</span>`;
    },

    badgeList(roleNames) {
        return (roleNames || []).map(n => RoleUtils.badge(n)).join("");
    },
};
