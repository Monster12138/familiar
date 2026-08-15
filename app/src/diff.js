// Line-based config diff shared by the settings panel and the onboarding
// page. `computeLineDiff` aligns before/after lines via longest-common-
// subsequence (git-diff style), so inserting one line no longer mislabels
// every following line as changed (the old index-based comparison did).

/**
 * Align two texts line by line using LCS.
 * @param {string} beforeText
 * @param {string} afterText
 * @returns {Array<{type: 'same'|'add'|'remove', before: string|null, after: string|null}>}
 */
export function computeLineDiff(beforeText, afterText) {
    const before = beforeText ? beforeText.split('\n') : [];
    const after = afterText ? afterText.split('\n') : [];

    const m = before.length;
    const n = after.length;
    const dp = Array.from({ length: m + 1 }, () => new Array(n + 1).fill(0));
    for (let i = m - 1; i >= 0; i--) {
        for (let j = n - 1; j >= 0; j--) {
            dp[i][j] =
                before[i] === after[j]
                    ? dp[i + 1][j + 1] + 1
                    : Math.max(dp[i + 1][j], dp[i][j + 1]);
        }
    }

    const rows = [];
    let i = 0;
    let j = 0;
    while (i < m && j < n) {
        if (before[i] === after[j]) {
            rows.push({ type: 'same', before: before[i], after: after[j] });
            i++;
            j++;
        } else if (dp[i + 1][j] >= dp[i][j + 1]) {
            rows.push({ type: 'remove', before: before[i], after: null });
            i++;
        } else {
            rows.push({ type: 'add', before: null, after: after[j] });
            j++;
        }
    }
    while (i < m) {
        rows.push({ type: 'remove', before: before[i], after: null });
        i++;
    }
    while (j < n) {
        rows.push({ type: 'add', before: null, after: after[j] });
        j++;
    }
    return rows;
}

/** Escape and color JSON tokens for display in a <pre> block. */
export function syntaxHighlightJSON(json) {
    if (typeof json !== 'string') {
        json = JSON.stringify(json, undefined, 2);
    }
    json = json.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
    return json.replace(
        /("(\\u[a-zA-Z0-9]{4}|\\[^u]|[^\\"])*"(\s*:)?|\b(true|false|null)\b|-?\d+(?:\.\d*)?(?:[eE][+\-]?\d+)?)/g,
        (match) => {
            let cls = 'json-number';
            if (/^"/.test(match)) {
                cls = /:$/.test(match) ? 'json-key' : 'json-string';
            } else if (/true|false/.test(match)) {
                cls = 'json-boolean';
            } else if (/null/.test(match)) {
                cls = 'json-null';
            }
            return '<span class="' + cls + '">' + match + '</span>';
        }
    );
}

/**
 * Build the HTML for the Before/After panes of a diff viewer, aligned by
 * `computeLineDiff`. Added lines are green, removed lines red.
 * @returns {{beforeHTML: string, afterHTML: string}}
 */
export function renderDiffRows(beforeText, afterText) {
    const rows = computeLineDiff(beforeText, afterText);
    let beforeHTML = '';
    let afterHTML = '';
    for (const row of rows) {
        const b = row.before !== null ? syntaxHighlightJSON(row.before) : '';
        const a = row.after !== null ? syntaxHighlightJSON(row.after) : '';
        const bClass = row.type === 'remove' ? 'diff-line diff-remove' : 'diff-line';
        const aClass = row.type === 'add' ? 'diff-line diff-add' : 'diff-line';
        beforeHTML += `<div class="${bClass}">${b}</div>`;
        afterHTML += `<div class="${aClass}">${a}</div>`;
    }
    return { beforeHTML, afterHTML };
}
