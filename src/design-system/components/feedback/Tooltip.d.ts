import { ReactNode } from 'react';

export interface TooltipProps {
  /** Text shown on hover/focus. */
  label: ReactNode;
  /** The trigger element the tooltip wraps. */
  children: ReactNode;
  /** Which side of the trigger the tip appears on. @default "top" */
  side?: 'top' | 'bottom' | 'left' | 'right';
  /** Hover delay in ms before showing. @default 250 */
  delay?: number;
}

export function Tooltip(props: TooltipProps): JSX.Element;
