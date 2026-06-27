(function () {
    'use strict';

    var RULES = [
        { key: 'length',  test: function (v) { return v.length >= 8; } },
        { key: 'lower',   test: function (v) { return /[a-z]/.test(v); } },
        { key: 'upper',   test: function (v) { return /[A-Z]/.test(v); } },
        { key: 'number',  test: function (v) { return /[0-9]/.test(v); } },
        { key: 'special', test: function (v) { return /[^A-Za-z0-9]/.test(v); } },
    ];

    // 5 rules → scores 0-5. Score 5 = all rules met = Strong.
    // Scores 1 and 2 both map to Weak so the label never jumps too fast.
    var LEVELS = [
        { label: '',       color: '',        bars: 0 },
        { label: 'Weak',   color: '#ef4444', bars: 1 },
        { label: 'Weak',   color: '#ef4444', bars: 1 },
        { label: 'Fair',   color: '#f97316', bars: 2 },
        { label: 'Good',   color: '#eab308', bars: 3 },
        { label: 'Strong', color: '#22c55e', bars: 4 },
    ];

    function score(value) {
        if (!value) return 0;
        var s = 0;
        for (var i = 0; i < RULES.length; i++) {
            if (RULES[i].test(value)) s++;
        }
        return s; // 0–5
    }

    function initPasswordStrength(inputId, meterId, reqsId) {
        var input = document.getElementById(inputId);
        var meter = document.getElementById(meterId);
        if (!input || !meter) return;

        var bars    = meter.querySelectorAll('.pwd-bar');
        var labelEl = meter.querySelector('.pwd-label');
        var reqs    = reqsId ? document.getElementById(reqsId) : null;

        function update() {
            var val = input.value;
            var s   = score(val);
            var lvl = LEVELS[s];

            bars.forEach(function (bar, i) {
                bar.style.background = i < lvl.bars ? lvl.color : '';
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
        }

        input.addEventListener('input', update);

        // Block submission unless all 5 rules pass (score === 5 = Strong)
        var form = input.closest('form');
        if (form) {
            form.addEventListener('submit', function (e) {
                if (score(input.value) < 5) {
                    e.preventDefault();
                    update();
                    meter.classList.add('pwd-error');
                    setTimeout(function () { meter.classList.remove('pwd-error'); }, 500);
                    input.focus();
                }
            });
        }
    }

    window.initPasswordStrength = initPasswordStrength;
})();
