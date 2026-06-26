/**
 * Coordinate transformation utilities for converting between screen space,
 * normalized fractions, and PDF points.
 *
 * PDF coordinates are in points (1/72 inch), measured from bottom-left.
 * Screen coordinates are in pixels, measured from top-left.
 * Fractions are normalized 0..1 within the overlay element.
 */

/**
 * Convert screen coordinates to normalized fractions (0..1) relative to an overlay element.
 *
 * @param clientX - Mouse X coordinate in screen space
 * @param clientY - Mouse Y coordinate in screen space
 * @param overlayRef - Reference to the overlay HTML element (position:absolute parent)
 * @returns Normalized coordinates {x, y} in range [0, 1], or null if overlay unavailable
 *
 * @example
 * const fraction = fractionFromEvent(event.clientX, event.clientY, overlayRef)
 * // Returns: { x: 0.5, y: 0.3 } (center-left of overlay)
 */
export function fractionFromEvent(
  clientX: number,
  clientY: number,
  overlayRef: React.RefObject<HTMLElement>
): { x: number; y: number } | null {
  const el = overlayRef.current
  if (!el) return null

  const rect = el.getBoundingClientRect()
  if (rect.width === 0 || rect.height === 0) return null

  return {
    x: Math.min(1, Math.max(0, (clientX - rect.left) / rect.width)),
    y: Math.min(1, Math.max(0, (clientY - rect.top) / rect.height)),
  }
}

/**
 * Convert normalized fractions to PDF points (user space coordinates).
 *
 * Fractions [0, 1] map to [0, pageWidth] and [0, pageHeight] in points.
 *
 * @param fractionX - Normalized X coordinate in range [0, 1]
 * @param fractionY - Normalized Y coordinate in range [0, 1]
 * @param pageWidthPts - Page width in PDF points
 * @param pageHeightPts - Page height in PDF points
 * @returns Coordinates in PDF points {x_pts, y_pts}
 *
 * @example
 * const pdfPts = fractionToPdfPoints(0.5, 0.3, 612, 792)
 * // Returns: { x_pts: 306, y_pts: 237.6 }
 */
export function fractionToPdfPoints(
  fractionX: number,
  fractionY: number,
  pageWidthPts: number,
  pageHeightPts: number
): { x_pts: number; y_pts: number } {
  return {
    x_pts: fractionX * pageWidthPts,
    y_pts: fractionY * pageHeightPts,
  }
}

/**
 * Convert screen coordinates directly to PDF points (user space).
 *
 * This combines fractionFromEvent + fractionToPdfPoints for convenience.
 *
 * @param clientX - Mouse X coordinate in screen space
 * @param clientY - Mouse Y coordinate in screen space
 * @param overlayRef - Reference to the overlay HTML element
 * @param pageWidthPts - Page width in PDF points
 * @param pageHeightPts - Page height in PDF points
 * @returns Coordinates in PDF points {x_pts, y_pts}, or null if conversion fails
 *
 * @example
 * const pdfPts = clientToPdfPoints(event.clientX, event.clientY, overlayRef, 612, 792)
 * // Returns: { x_pts: 306, y_pts: 237.6 } or null
 */
export function clientToPdfPoints(
  clientX: number,
  clientY: number,
  overlayRef: React.RefObject<HTMLElement>,
  pageWidthPts: number,
  pageHeightPts: number
): { x_pts: number; y_pts: number } | null {
  const fraction = fractionFromEvent(clientX, clientY, overlayRef)
  if (!fraction) return null

  return fractionToPdfPoints(fraction.x, fraction.y, pageWidthPts, pageHeightPts)
}

/**
 * Calculate the size of a rectangle defined by two diagonal fractions.
 *
 * @param x0 - Start X in fractions
 * @param y0 - Start Y in fractions
 * @param x1 - End X in fractions
 * @param y1 - End Y in fractions
 * @returns Normalized rectangle {x, y, width, height} all in fractions [0, 1]
 */
export function normalizeFractionRect(
  x0: number,
  y0: number,
  x1: number,
  y1: number
): { x: number; y: number; width: number; height: number } {
  const minX = Math.min(x0, x1)
  const maxX = Math.max(x0, x1)
  const minY = Math.min(y0, y1)
  const maxY = Math.max(y0, y1)

  return {
    x: minX,
    y: minY,
    width: maxX - minX,
    height: maxY - minY,
  }
}
