import * as React from 'react';

export interface DialogProps {
  /** Visibility. */
  open: boolean;
  /** Called on scrim click, Escape, or close button. */
  onClose?: () => void;
  /** Header title. */
  title?: React.ReactNode;
  /** Supporting line under the title. */
  description?: React.ReactNode;
  /** Footer actions (right-aligned), usually Buttons. */
  footer?: React.ReactNode;
  /** Pixel width of the panel. @default 480 */
  width?: number;
  /** Dismiss when the scrim is clicked. @default true */
  closeOnScrim?: boolean;
  children?: React.ReactNode;
}

/**
 * Centered modal dialog with scrim + blur, pop-in animation, and Escape/scrim
 * dismissal. Use for confirmations, quick forms (new order, record payment).
 */
export function Dialog(props: DialogProps): JSX.Element | null;
