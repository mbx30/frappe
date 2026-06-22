@@
-import { useState, useEffect } from 'react'
-import { invoke } from '@tauri-apps/api/core'
+import { useState, useEffect } from 'react'
+import { invoke } from '@tauri-apps/api/core'
@@
-export default function OrderList({ onCreateNew, onSelectOrder }: OrderListProps) {
-  const [orders, setOrders] = useState<Order[]>([])
-  const [isLoading, setIsLoading] = useState(true)
-
-  useEffect(() => {
-    loadOrders()
-  }, [])
-
-  const loadOrders = async () => {
-    try {
-      const result = await invoke<Order[]>('list_orders')
-      setOrders(result)
-    } catch (e) {
-      console.error('Failed to load orders:', e)
-    } finally {
-      setIsLoading(false)
-    }
-  }
+export default function OrderList({ onCreateNew, onSelectOrder }: OrderListProps) {
+  const [orders, setOrders] = useState<Order[]>([])
+  const [isLoading, setIsLoading] = useState(true)
+
+  // Hoisted function declaration (function declarations are hoisted)
+  async function loadOrders() {
+    try {
+      const result = await invoke<Order[]>('list_orders')
+      setOrders(result)
+    } catch (e) {
+      console.error('Failed to load orders:', e)
+    } finally {
+      setIsLoading(false)
+    }
+  }
+
+  useEffect(() => {
+    loadOrders()
+  }, [])
*** End Patch
