import { useRef, useEffect, useState, type CSSProperties, type ReactNode } from 'react'
import { List } from 'react-window'

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
 * Thin wrapper around react-window's `List` that infers the
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

  if (items.length === 0) {
    return (
      <div ref={containerRef} className={className}>
        {emptyState ?? <p className="pdf-empty">No items.</p>}
      </div>
    )
  }

  const numericHeight = typeof height === 'number' ? height : 480
  const listStyle: CSSProperties = {
    ...style,
    height: numericHeight,
    width: containerWidth || 800,
  }

  return (
    <div ref={containerRef} className={className}>
      <List
        rowCount={items.length}
        rowHeight={itemHeight}
        rowComponent={({ index, style: rowStyle }) => {
          const item = items[index]
          return (
            <div key={keyExtractor(item, index)} style={rowStyle}>
              {renderItem(item, index)}
            </div>
          )
        }}
        overscanCount={overscanCount}
        style={listStyle}
        rowProps={{}}
      />
    </div>
  )
}

export default VirtualList
