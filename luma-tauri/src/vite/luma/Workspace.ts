export class Workspace {
    public project_uuid: string;
    public title: string;
    public path: string;
    public icon?: string;
    public color: string;
    public git_branch?: string;
    constructor(project_uuid: string, title: string, path: string, icon?: string, git_branch?: string) {
        this.project_uuid = project_uuid;
        this.title = title;
        this.path = path;
        this.icon = icon;
        this.color = getProjectColor(title);
        this.git_branch = git_branch;
    }
}
export function getProjectInitials(title: string) {
    const words = title.trim().split(/\s+/).filter(Boolean);

    if (words.length === 0) return "";

    if (words.length === 1) {
        return [...words[0]].slice(0, 2).join("").toUpperCase();
    }

    return words
        .slice(0, 2)
        .map(word => [...word][0])
        .join("")
        .toUpperCase();
}


export function getProjectColor(title: string) {
    let hash = 0;

    for (let i = 0; i < title.length; i++) {
        hash = title.charCodeAt(i) + ((hash << 5) - hash);
        hash |= 0;
    }

    const hue = Math.abs(hash) % 360;

    return `hsl(${hue}, 65%, 45%)`;
}
