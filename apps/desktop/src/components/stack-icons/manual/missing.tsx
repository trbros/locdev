import type { SVGProps } from "react";

/** Sem logo verificado: indicador neutro, não uma marca inventada. */
export default function MissingStackIcon(props: SVGProps<SVGSVGElement>) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.7"
      strokeLinecap="round"
      strokeLinejoin="round"
      {...props}
    >
      <path d="M12 2.8 20.5 7.5v9L12 21.2l-8.5-4.7v-9L12 2.8Z" />
      <path d="M3.5 7.5 12 12l8.5-4.5M12 12v9.2" />
    </svg>
  );
}
