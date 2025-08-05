/**
 * File Drop Zone Component for Vis System
 *
 * Handles file upload via drag-and-drop or file input.
 * Integrates with the new vis system's JSON parser.
 */
interface FileDropZoneProps {
    onFileLoad: (data: any) => void;
    hasData?: boolean;
    className?: string;
}
export declare function FileDropZone({ onFileLoad, hasData, className }: FileDropZoneProps): import("react/jsx-runtime").JSX.Element;
export default FileDropZone;
//# sourceMappingURL=FileDropZone.d.ts.map