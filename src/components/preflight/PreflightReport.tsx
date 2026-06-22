@@
-  const [sections, setSections] = useState<SectionState>({
-    fonts: true, boxes: true, images: true, bleed: true, intents: true, security: true, pdfx: true, color_spaces: true, spot_colors: true, overprint: true, transparency: true, hidden_content: true[...]
-  })
+  const [sections, setSections] = useState<SectionState>({
+    fonts: true, boxes: true, images: true, bleed: true, intents: true, security: true, pdfx: true, color_spaces: true, spot_colors: true, overprint: true, transparency: true, hidden_content: true
+  })
@@
-  const handleSave = async () => {
-    if (!jobId) return
-    setSaving(true)
-    setSaveMsg(null)
-    try {
-      const findings: any[] = []
-      for (const f of result.fonts) findings.push({ check_name: 'fonts', severity: f.severity, page_num: null, object_ref: null, message: f.message, fix_hint: '' })
-      for (const f of result.page_boxes) findings.push({ check_name: 'page_boxes', severity: f.severity, page_num: f.page as any, object_ref: null, message: f.message, fix_hint: '' })
-      for (const f of result.images) findings.push({ check_name: 'image_resolution', severity: f.severity, page_num: f.page as any, object_ref: f.image_name, message: f.message, fix_hint: '' })
-      for (const f of bleedFindings) findings.push({ check_name: 'bleed', severity: f.severity, page_num: f.page as any, object_ref: null, message: f.message, fix_hint: '' })
-      for (const f of result.security) findings.push({ check_name: 'security', severity: f.severity, page_num: null, object_ref: f.category, message: f.message, fix_hint: '' })
-      for (const f of result.pdfx) findings.push({ check_name: 'pdfx', severity: f.severity, page_num: null, object_ref: f.category, message: f.message, fix_hint: f.fix_hint })
-      for (const f of result.color_spaces) findings.push({ check_name: 'color_spaces', severity: f.severity, page_num: null, object_ref: f.color_space, message: f.message, fix_hint: '' })
-      for (const f of result.overprint) findings.push({ check_name: 'overprint', severity: f.severity, page_num: f.page as any, object_ref: f.object_context, message: f.message, fix_hint: '' })
-      for (const f of result.transparency) findings.push({ check_name: 'transparency', severity: f.severity, page_num: f.page as any, object_ref: f.ty, message: f.message, fix_hint: '' })
-      for (const f of result.hidden_content) findings.push({ check_name: 'hidden_content', severity: f.severity, page_num: f.page as any, object_ref: f.ty, message: f.description, fix_hint: '' })
-      await invoke('save_preflight_run', { jobId, profile, findings })
-      setSaveMsg('Report saved!')
-      onSaved()
-    } catch (e) {
-      setSaveMsg(`Save failed: ${e}`)
-    } finally {
-      setSaving(false)
-    }
-  }
+  const handleSave = async () => {
+    if (!jobId) return
+    setSaving(true)
+    setSaveMsg(null)
+    try {
+      const findings: Array<Record<string, unknown>> = []
+      for (const f of result.fonts) findings.push({ check_name: 'fonts', severity: f.severity, page_num: null, object_ref: null, message: f.message, fix_hint: '' })
+      for (const f of result.page_boxes) findings.push({ check_name: 'page_boxes', severity: f.severity, page_num: typeof f.page === 'number' ? f.page : null, object_ref: null, message: f.message, fix_hint: '' })
+      for (const f of result.images) findings.push({ check_name: 'image_resolution', severity: f.severity, page_num: typeof f.page === 'number' ? f.page : null, object_ref: f.image_name, message: f.message, fix_hint: '' })
+      for (const f of bleedFindings) findings.push({ check_name: 'bleed', severity: f.severity, page_num: typeof f.page === 'number' ? f.page : null, object_ref: null, message: f.message, fix_hint: '' })
+      for (const f of result.security) findings.push({ check_name: 'security', severity: f.severity, page_num: null, object_ref: f.category, message: f.message, fix_hint: '' })
+      for (const f of result.pdfx) findings.push({ check_name: 'pdfx', severity: f.severity, page_num: null, object_ref: f.category, message: f.message, fix_hint: f.fix_hint })
+      for (const f of result.color_spaces) findings.push({ check_name: 'color_spaces', severity: f.severity, page_num: null, object_ref: f.color_space, message: f.message, fix_hint: '' })
+      for (const f of result.overprint) findings.push({ check_name: 'overprint', severity: f.severity, page_num: typeof f.page === 'number' ? f.page : null, object_ref: f.object_context, message: f.message, fix_hint: '' })
+      for (const f of result.transparency) findings.push({ check_name: 'transparency', severity: f.severity, page_num: typeof f.page === 'number' ? f.page : null, object_ref: f.ty, message: f.message, fix_hint: '' })
+      for (const f of result.hidden_content) findings.push({ check_name: 'hidden_content', severity: f.severity, page_num: typeof f.page === 'number' ? f.page : null, object_ref: f.ty, message: f.description, fix_hint: '' })
+      await invoke('save_preflight_run', { jobId, profile, findings })
+      setSaveMsg('Report saved!')
+      onSaved()
+    } catch (e) {
+      setSaveMsg(`Save failed: ${e}`)
+    } finally {
+      setSaving(false)
+    }
+  }
*** End Patch
