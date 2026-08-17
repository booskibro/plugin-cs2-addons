// Version comparison for update badges.
//
// Version strings for the same build arrive in different costumes:
// CounterStrikeSharp reports "1.0.371 @ 3923c5d" while its release tag is
// "v1.0.371"; Metamod reports "2.0.0-dev+1410" while its build file is
// named "2.0.0-git1410". Comparing raw strings therefore produces false
// "update available" badges for identical builds. The stable identity is
// the sequence of numbers: take the first token (before whitespace or a
// commit-hash "@"), then keep only the digit groups.

export function versionKey(raw: string): string {
    const head = raw.trim().toLowerCase().replace(/^v/, '').split(/[\s@]+/)[0] ?? '';
    const digits = head.match(/\d+/g);
    return digits ? digits.join('.') : head;
}

/** True when both strings describe the same build. */
export function versionsMatch(a: string, b: string): boolean {
    const keyA = versionKey(a);
    const keyB = versionKey(b);
    if (keyA === '' || keyB === '') {
        return a.trim().toLowerCase() === b.trim().toLowerCase();
    }
    return keyA === keyB;
}
