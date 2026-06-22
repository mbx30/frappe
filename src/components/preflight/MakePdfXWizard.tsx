@@
   const handleFetchSpotColors = async () => {
     try {
       const spots = await invoke<SpotColorFinding[]>('check_spot_colors', { path: filePath })
       setSpotColors(spots)
     } catch { }
   }
@@
-      if (fixBleed && hasBleedIssues) {
+      if (fixBleed && hasBleedIssues) {
         const bleedOut = `${base}_bleed.pdf`
         await invoke('add_bleed', { path: currentPath, amountMm: 3, outputPath: bleedOut })
         currentPath = bleedOut
         steps.push('Added bleed (3mm)')
       }
@@
-      if (fixColors && hasRgbContent) {
+      if (fixColors && hasRgbContent) {
         const cmykOut = `${base}_cmyk.pdf`
         await invoke('convert_rgb_to_cmyk', {
           path: currentPath,
           outputPath: cmykOut,
           scope: 'both',
           srcProfile: null,
           dstProfile: null,
           renderingIntent: null,
         })
         currentPath = cmykOut
         steps.push('Converted RGB→CMYK')
       }
@@
-      if (fixOutputIntent && hasNoOutputIntent && (profile === 'x4' || profile === 'x1a')) {
+      if (fixOutputIntent && hasNoOutputIntent && (profile === 'x4' || profile === 'x1a')) {
         const intentOut = `${base}_pdfx.pdf`
         const conditionId = profile === 'x4' ? 'FOGRA39 (ISO Coated v2)' : 'FOGRA39 (ISO Coated v2)'
         const condition = profile === 'x4' ? 'ISO Coated v2 (FOGRA39)' : 'ISO Coated v2 (FOGRA39)'
         await invoke('add_output_intent', {
           path: currentPath,
           outputPath: intentOut,
           iccProfile: '',
           conditionId,
           condition,
         })
         currentPath = intentOut
         steps.push(`Added ${profile === 'x4' ? 'PDF/X-4' : 'PDF/X-1a'} OutputIntent`)
       }
@@
-    } catch (e) {
-      setError(String(e))
-    } finally {
-      setApplying(false)
-    }
+    } catch (e) {
+      setError(String(e))
+    } finally {
+      setApplying(false)
+    }
   }
*** End Patch