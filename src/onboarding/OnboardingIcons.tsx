import type { SVGProps } from "react";

type IconProps = SVGProps<SVGSVGElement> & {
  size?: number;
};

function baseProps({ size = 18, ...props }: IconProps) {
  return {
    width: size,
    height: size,
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 1.6,
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    "aria-hidden": true,
    focusable: false,
    ...props,
  };
}

export function DocumentIcon(props: IconProps) {
  return (
    <svg {...baseProps(props)}>
      <path d="M6.5 3.5h7l4 4v13h-11z" />
      <path d="M13.5 3.5v4h4" />
      <path d="M9 12h6M9 15.5h4" />
    </svg>
  );
}

export function ImageIcon(props: IconProps) {
  return (
    <svg {...baseProps(props)}>
      <rect x="3.5" y="4.5" width="17" height="15" rx="2.2" />
      <circle cx="9" cy="10" r="1.7" />
      <path d="m4.5 17 4.3-4.1 3.1 2.8 2.6-2.4 4.9 4.5" />
    </svg>
  );
}

export function AudioIcon(props: IconProps) {
  return (
    <svg {...baseProps(props)}>
      <path d="M4 14.5v-5M8 17.5v-11M12 15.5v-7M16 18.5v-13M20 14.5v-5" />
    </svg>
  );
}

export function VideoIcon(props: IconProps) {
  return (
    <svg {...baseProps(props)}>
      <rect x="3.5" y="5.5" width="17" height="13" rx="2.2" />
      <path d="m10 9.3 4.8 2.7-4.8 2.7z" />
    </svg>
  );
}

export function ArchiveIcon(props: IconProps) {
  return (
    <svg {...baseProps(props)}>
      <path d="M4.5 7.5h15v12h-15z" />
      <path d="M3.5 4.5h17v3h-17zM12 10v2M12 14v2" />
    </svg>
  );
}

export function FolderIcon(props: IconProps) {
  return (
    <svg {...baseProps(props)}>
      <path d="M3.5 6.5h6l2 2h9v11h-17z" />
      <path d="M3.5 8.5v11" />
    </svg>
  );
}

export function KeyIcon(props: IconProps) {
  return (
    <svg {...baseProps(props)}>
      <circle cx="8.2" cy="12" r="3.4" />
      <path d="M11.6 12H21M17.2 12v3M19.6 12v2" />
    </svg>
  );
}

export function PaletteIcon(props: IconProps) {
  return (
    <svg {...baseProps(props)}>
      <path d="M12 3.5a8.5 8.5 0 1 0 0 17c1.5 0 2.1-.9 2.1-1.9 0-1.5-1.5-2.1-1.5-3.4 0-1.1.9-1.9 2.1-1.9h1.5c2.1 0 3.3-1.3 3.3-3.1 0-3.8-3.4-6.7-7.5-6.7Z" />
      <circle cx="8" cy="10" r=".8" fill="currentColor" stroke="none" />
      <circle cx="10.2" cy="7.2" r=".8" fill="currentColor" stroke="none" />
      <circle cx="14" cy="7.2" r=".8" fill="currentColor" stroke="none" />
    </svg>
  );
}

export function WindowsIcon(props: IconProps) {
  return (
    <svg {...baseProps(props)}>
      <path d="M4 5.8 10.5 5v6H4zM12 4.8 20 3.5v7.5h-8zM4 13h6.5v6L4 18.2zM12 13h8v7.5L12 19.2z" />
    </svg>
  );
}

export function FileIcon(props: IconProps) {
  return (
    <svg {...baseProps(props)}>
      <path d="M7 3.5h6.8L18 7.8v12.7H7z" />
      <path d="M13.8 3.5v4.3H18" />
    </svg>
  );
}

export function ArrowLeftIcon(props: IconProps) {
  return (
    <svg {...baseProps(props)}>
      <path d="m14.5 6-6 6 6 6" />
    </svg>
  );
}

export function ArrowRightIcon(props: IconProps) {
  return (
    <svg {...baseProps(props)}>
      <path d="m9.5 6 6 6-6 6" />
    </svg>
  );
}

export function CheckIcon(props: IconProps) {
  return (
    <svg {...baseProps(props)}>
      <path d="m5.5 12.5 4.2 4.1 8.8-9.2" />
    </svg>
  );
}

export function CloseIcon(props: IconProps) {
  return (
    <svg {...baseProps(props)}>
      <path d="m6.5 6.5 11 11M17.5 6.5l-11 11" />
    </svg>
  );
}

export function SparkIcon(props: IconProps) {
  return (
    <svg {...baseProps(props)}>
      <path d="M12 3.5c.7 5.2 2.8 7.3 8 8-5.2.7-7.3 2.8-8 8-.7-5.2-2.8-7.3-8-8 5.2-.7 7.3-2.8 8-8Z" />
    </svg>
  );
}

export function LockIcon(props: IconProps) {
  return (
    <svg {...baseProps(props)}>
      <rect x="5" y="10" width="14" height="10" rx="2" />
      <path d="M8.5 10V7.8a3.5 3.5 0 0 1 7 0V10" />
    </svg>
  );
}
