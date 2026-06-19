import * as React from 'react';

export interface ToastProps {
  title?: React.ReactNode;
  message?: React.ReactNode;
  /** Semantic accent + icon. @default "neutral" */
  tone?: 'neutral' | 'success' | 'warning' | 'danger' | 'info';
  /** Auto-dismiss after N ms; 0 = sticky. @default 4000 */
  duration?: number;
  /** Optional action node (e.g. an "Undo" Button). */
  action?: React.ReactNode;
  onClose?: () => void;
}

export interface ToastViewportProps {
  /** @default "bottom-right" */
  placement?: 'bottom-right' | 'bottom-left' | 'top-right' | 'top-center';
  children?: React.ReactNode;
}

/** A single transient notification card with a colored accent edge. */
export function Toast(props: ToastProps): JSX.Element;
/** Fixed stacking container for toasts — render once near the app root. */
export function ToastViewport(props: ToastViewportProps): JSX.Element;
