import * as React from 'react';

export interface TabItem {
  value: string;
  label: React.ReactNode;
  /** Optional leading icon. */
  icon?: React.ReactNode;
  /** Optional trailing count pill. */
  count?: number;
}

export interface TabsProps {
  /** Tabs as `{value,label,icon?,count?}` or plain strings. */
  tabs: (TabItem | string)[];
  /** Controlled active value. */
  value?: string;
  /** Uncontrolled initial value. */
  defaultValue?: string;
  /** Fires with the new active value. */
  onChange?: (value: string) => void;
  /** `underline` for page nav, `pill` for segmented sub-views. @default "underline" */
  variant?: 'underline' | 'pill';
  /** @default "md" */
  size?: 'sm' | 'md';
  style?: React.CSSProperties;
}

/**
 * Tab strip with an `underline` look for top-level page navigation and a
 * `pill` (segmented) look for in-panel filters. Supports icons and count pills.
 */
export function Tabs(props: TabsProps): JSX.Element;
