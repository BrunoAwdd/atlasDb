import { CheckCircle, XCircle, Clock, AlertTriangle } from "lucide-react";

export type StatusType = "success" | "pending" | "failed" | "warning";

interface StatusBadgeProps {
  status: StatusType;
  text?: string;
  className?: string;
}

const config = {
  success: {
    icon: CheckCircle,
    bg: "bg-green-500/20",
    text: "text-green-400",
    label: "Success",
  },
  pending: {
    icon: Clock,
    bg: "bg-yellow-500/20",
    text: "text-yellow-400",
    label: "Pending",
  },
  failed: {
    icon: XCircle,
    bg: "bg-red-500/20",
    text: "text-red-400",
    label: "Failed",
  },
  warning: {
    icon: AlertTriangle,
    bg: "bg-orange-500/20",
    text: "text-orange-400",
    label: "Warning",
  },
};

export default function StatusBadge({
  status,
  text,
  className = "",
}: StatusBadgeProps) {
  const {
    icon: Icon,
    bg,
    text: textColor,
    label,
  } = config[status] || config.pending;

  return (
    <span
      className={`inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-medium ${bg} ${textColor} ${className}`}
    >
      <Icon size={12} />
      <span>{text || label}</span>
    </span>
  );
}
