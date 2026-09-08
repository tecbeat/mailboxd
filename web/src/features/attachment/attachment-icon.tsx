import { cn } from "@/lib/utils";
import { getFileTypeConfig } from "@/lib/file-type";

interface AttachmentIconProps {
    contentType: string;
    className?: string;
}

export function AttachmentIcon({ contentType, className }: AttachmentIconProps) {
    const { Icon, text } = getFileTypeConfig(contentType);

    return (
        <Icon
            className={cn("h-4 w-4 shrink-0 opacity-90", text, className)}
            strokeWidth={2}
        />
    );
}
