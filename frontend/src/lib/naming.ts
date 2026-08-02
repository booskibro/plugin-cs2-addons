// Name helpers for plugin folders and uploads.

export function fileExtension(fileName: string): string {
    const idx = fileName.lastIndexOf('.');
    return idx > 0 ? fileName.slice(idx + 1).toLowerCase() : '';
}

export function fileStem(fileName: string): string {
    const idx = fileName.lastIndexOf('.');
    return idx > 0 ? fileName.slice(0, idx) : fileName;
}

/**
 * Human-ish plugin name from a folder name:
 * high_ping_kicker → High Ping Kicker, WeaponPaints → Weapon Paints,
 * CS2-Tags → CS2 Tags.
 */
export function prettyName(folder: string): string {
    return folder
        .replace(/[_-]+/g, ' ')
        .replace(/([a-z0-9])([A-Z])/g, '$1 $2')
        .replace(/\s+/g, ' ')
        .trim()
        .replace(/(^|\s)[a-z]/g, (ch) => ch.toUpperCase());
}
