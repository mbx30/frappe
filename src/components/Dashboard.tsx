@@
-import { useState, useEffect } from 'react'
+import { useState, useEffect, useCallback } from 'react'
@@
-export default function Dashboard() {
+export default function Dashboard() {
@@
-  useEffect(() => {
-    loadOrders()
-  }, [])
-
-  useEffect(() => {
-    applyFilters()
-  }, [orders, searchText, filterStatus, filterPriority])
-
-  const loadOrders = async () => {
-    setIsLoading(true)
-    setLoadError(null)
-    try {
-      const result = await invoke<Order[]>('list_orders')
-      setOrders(result)
-      calculateStats(result)
-    } catch (e) {
-      console.error('Failed to load orders:', e)
-      setLoadError(String(e))
-    } finally {
-      setIsLoading(false)
-    }
-  }
-
-  const calculateStats = (orderList: Order[]) => {
-    const today = new Date().toISOString().split('T')[0]
-    let overdue = 0
-    let dueToday = 0
-
-    orderList.filter((o) => o.status !== 'completed').forEach((o) => {
-      if (o.due_date < today) overdue++
-      if (o.due_date === today) dueToday++
-    })
-
-    setStats({
-      total: orderList.length,
-      prepress: orderList.filter((o) => o.status === 'prepress').length,
-      production: orderList.filter((o) => o.status === 'production').length,
-      delivery: orderList.filter((o) => o.status === 'delivery').length,
-      completed: orderList.filter((o) => o.status === 'completed').length,
-      overdue,
-      dueToday,
-    })
-  }
-
-  const applyFilters = () => {
-    let filtered = [...orders]
-
-    if (searchText) {
-      const search = searchText.toLowerCase()
-      filtered = filtered.filter(
-        (o) =>
-          o.order_number.toLowerCase().includes(search) ||
-          o.description.toLowerCase().includes(search)
-      )
-    }
-
-    if (filterStatus) {
-      filtered = filtered.filter((o) => o.status === filterStatus)
-    }
-
-    if (filterPriority) {
-      filtered = filtered.filter((o) => o.priority === filterPriority)
-    }
-
-    setFilteredOrders(filtered)
-  }
+  // calculateStats - stable identity
+  const calculateStats = useCallback((orderList: Order[]) => {
+    const today = new Date().toISOString().split('T')[0]
+    let overdue = 0
+    let dueToday = 0
+
+    orderList.filter((o) => o.status !== 'completed').forEach((o) => {
+      if (o.due_date < today) overdue++
+      if (o.due_date === today) dueToday++
+    })
+
+    setStats({
+      total: orderList.length,
+      prepress: orderList.filter((o) => o.status === 'prepress').length,
+      production: orderList.filter((o) => o.status === 'production').length,
+      delivery: orderList.filter((o) => o.status === 'delivery').length,
+      completed: orderList.filter((o) => o.status === 'completed').length,
+      overdue,
+      dueToday,
+    })
+  }, [])
+
+  // loadOrders - stable identity and safe to reference in effects/props
+  const loadOrders = useCallback(async () => {
+    setIsLoading(true)
+    setLoadError(null)
+    try {
+      const result = await invoke<Order[]>('list_orders')
+      setOrders(result)
+      calculateStats(result)
+    } catch (e) {
+      console.error('Failed to load orders:', e)
+      setLoadError(String(e))
+    } finally {
+      setIsLoading(false)
+    }
+  }, [calculateStats])
+
+  // applyFilters uses latest order/filter state
+  const applyFilters = useCallback(() => {
+    let filtered = [...orders]
+
+    if (searchText) {
+      const search = searchText.toLowerCase()
+      filtered = filtered.filter(
+        (o) =>
+          o.order_number.toLowerCase().includes(search) ||
+          o.description.toLowerCase().includes(search)
+      )
+    }
+
+    if (filterStatus) {
+      filtered = filtered.filter((o) => o.status === filterStatus)
+    }
+
+    if (filterPriority) {
+      filtered = filtered.filter((o) => o.priority === filterPriority)
+    }
+
+    setFilteredOrders(filtered)
+  }, [orders, searchText, filterStatus, filterPriority])
+
+  // Effects now reference stable callbacks (no "accessed before declared" or missing deps)
+  useEffect(() => {
+    loadOrders()
+  }, [loadOrders])
+
+  useEffect(() => {
+    applyFilters()
+  }, [applyFilters])
*** End Patch
