import { formatDistanceToNow, format } from 'date-fns';

export function formatRelativeTime(ts: number): string {
  return formatDistanceToNow(new Date(ts * 1000), { addSuffix: true });
}

export function formatDateTime(ts: number): string {
  return format(new Date(ts * 1000), 'yyyy-MM-dd HH:mm:ss');
}

export function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

export function truncate(text: string, maxLen: number): string {
  if (text.length <= maxLen) return text;
  return text.slice(0, maxLen) + '...';
}
