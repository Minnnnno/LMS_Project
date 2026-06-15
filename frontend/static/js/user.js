// user.js — User display helpers (depends on lms-core.js)

window.UserUtils = {
    initials(firstName, lastName) {
        return ((firstName || "")[0] || "").toUpperCase()
             + ((lastName  || "")[0] || "").toUpperCase();
    },

    fullName(user) {
        return [user.first_name, user.last_name].filter(Boolean).join(" ")
            || user.email
            || "Unknown";
    },
};
