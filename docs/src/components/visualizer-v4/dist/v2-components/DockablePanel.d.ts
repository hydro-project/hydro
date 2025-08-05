export function DockablePanel({ id, title, children, defaultPosition, defaultDocked, defaultCollapsed, onPositionChange, className, minWidth, minHeight }: {
    id: any;
    title: any;
    children: any;
    defaultPosition?: string;
    defaultDocked?: boolean;
    defaultCollapsed?: boolean;
    onPositionChange: any;
    className?: string;
    minWidth?: number;
    minHeight?: number;
}): import("react/jsx-runtime").JSX.Element;
export namespace DOCK_POSITIONS {
    let TOP_LEFT: string;
    let TOP_RIGHT: string;
    let BOTTOM_LEFT: string;
    let BOTTOM_RIGHT: string;
    let FLOATING: string;
}
//# sourceMappingURL=DockablePanel.d.ts.map