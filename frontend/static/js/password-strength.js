(function () {
    'use strict';

    var LEVELS = [
        { label: '',       color: '' },
        { label: 'Weak',   color: '#ef4444' },
        { label: 'Fair',   color: '#f97316' },
        { label: 'Good',   color: '#eab308' },
        { label: 'Strong', color: '#22c55e' },
    ];

    var RULES = [
        { key: 'length',  test: function (v) { return v.length >= 8; } },
        { key: 'lower',   test: function (v) { return /[a-z]/.test(v); } },
        { key: 'upper',   test: function (v) { return /[A-Z]/.test(v); } },
        { key: 'number',  test: function (v) { return /[0-9]/.test(v); } },
        { key: 'special', test: function (v) { return /[^A-Za-z0-9]/.test(v); } },
    ];

    function score(value) {
        if (!value) return 0;
        var s = 0;
        for (var i = 0; i < RULES.length; i++) {
            if (RULES[i].test(value)) s++;
        }
        return Math.min(s, 4);
    }

    function initPasswordStrength(inputId, meterId, reqsId) {
        var input = document.getElementById(inputId);
        var meter = document.getElementById(meterId);
        if (!input || !meter) return;

        var bars    = meter.querySelectorAll('.pwd-bar');
        var labelEl = meter.querySelector('.pwd-label');
        var reqs    = reqsId ? document.getElementById(reqsId) : null;

        input.addEventListener('input', function () {
            var val = this.value;
            var s   = score(val);
            var lvl = LEVELS[s];

            bars.forEach(function (bar, i) {
                bar.style.background = i < s ? lvl.color : '';
            });

            if (labelEl) {
                labelEl.textContent = val ? lvl.label : '';
                labelEl.style.color = lvl.color;
            }

            if (reqs) {
                RULES.forEach(function (rule) {
                    var el = reqs.querySelector('[data-rule="' + rule.key + '"]');
                    if (el) {
                        el.classList.toggle('passed', !!val && rule.test(val));
                    }
                });
            }
        });
    }

    window.initPasswordStrength = initPasswordStrength;
})();
