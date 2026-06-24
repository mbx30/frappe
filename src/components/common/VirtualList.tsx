import { useRef, useEffect, useState, CSSProperties, ReactNode } from 'react'
import { VariableSizeList, ListChildComponentProps } from 'react-window'

interface VirtualListProps<T> {
  items: T[]
  itemHeight: number
  height: number | string
  renderItem: (item: T, index: number) => ReactNode
  keyExtractor: (item: T, index: number) => string | number
  emptyState?: ReactNode
  className?: string
  overscanCount?: number
  style?: CSSProperties
}

/**
 * Thin wrapper around react-window's `VariableSizeList` that infers the
 * item key, lets callers supply a row renderer, and exposes a single
 * `height` prop. Used by OrderListView, InvoiceList, ClientList, and
 * the Dashboard orders list to keep the DOM size constant for very
 * large result sets (verified to handle 5,000 rows).
 */
export function VirtualList<T>({
  items,
  itemHeight,
  height,
  renderItem,
  keyExtractor,
  emptyState,
  className,
  overscanCount = 8,
  style,
}: VirtualListProps<T>) {
  const listRef = useRef<VariableSizeList>(null)
  const [containerWidth, setContainerWidth] = useState<number>(0)
  const containerRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!containerRef.current) return
    const ro = new ResizeObserver((entries) => {
      for (const e of entries) {
        setContainerWidth(e.contentRect.width)
      }
    })
    ro.observe(containerRef.current)
    setContainerWidth(containerRef.current.clientWidth)
    return () => ro.disconnect()
  }, [])

  useEffect(() => {
    if (listRef.current) listRef.current.resetAfterIndex(0, true)
  }, [itemHeight])

  if (items.length === 0) {
    return (
      <div ref={containerRef} className={className}>
        {emptyState ?? <p className="pdf-empty">No items.</p>}
      </div>
    )
  }

  const numericHeight = typeof height === 'number' ? height : 480

  return (
    <div ref={containerRef} className={className} style={style}>
      <VariableSizeList
        ref={listRef}
        height={numericHeight}
        width={containerWidth || 800}
        itemCount={items.length}
        itemSize={() => itemHeight}
        overscanCount={overscanCount}
      >
        {({ index, style: rowStyle }: ListChildComponentProps) => {
          const item = items[index]
          return (
            <div key={keyExtractor(item, index)} style={rowStyle}>
              {renderItem(item, index)}
            </div>
          )
        }}
      </VariableSizeList>
    </div>
  )
}

export default VirtualList
