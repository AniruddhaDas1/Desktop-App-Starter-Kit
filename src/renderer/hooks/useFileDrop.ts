import { useCallback, useEffect, useRef, useState } from 'react';
import type { DragEvent } from 'react';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { blessDroppedPath } from '@lib/bridge';
import { useLogger } from './useLogger';
import type { DndResolvedPath } from '@shared/ipc/types';

export type { DndResolvedPath };

export interface UseFileDropOptions {
  onDrop: (paths: DndResolvedPath[]) => void | Promise<void>;
  disabled?: boolean;
}

export interface UseFileDropBind {
  onDragEnter: (e: DragEvent) => void;
  onDragOver: (e: DragEvent) => void;
  onDragLeave: (e: DragEvent) => void;
  onDrop: (e: DragEvent) => void;
}

export interface UseFileDropResult {
  isDragOver: boolean;
  bind: UseFileDropBind;
}

/**
 * Drag-and-drop for file paths.
 *
 * Tauri delivers drops at the webview level with absolute paths; a DOM `File`
 * can never be mapped back to a path. So the real work happens in
 * `onDragDropEvent`, and the DOM handlers stay only so call sites can keep
 * spreading `bind` for hover/cursor styling.
 *
 * Native drag-drop swallows HTML5 drop events on Windows. All drops must go
 * through this hook, never `dataTransfer.files`.
 */
export function useFileDrop({ onDrop, disabled = false }: UseFileDropOptions): UseFileDropResult {
  const [isDragOver, setIsDragOver] = useState(false);
  const log = useLogger('useFileDrop');
  const onDropRef = useRef(onDrop);

  useEffect(() => {
    onDropRef.current = onDrop;
  }, [onDrop]);

  useEffect(() => {
    if (disabled) return;
    let disposed = false;
    let unlisten: (() => void) | null = null;

    void getCurrentWebview()
      .onDragDropEvent(async (event) => {
        if (disposed) return;
        const payload = event.payload;

        if (payload.type === 'over') {
          setIsDragOver(true);
          return;
        }
        if (payload.type === 'leave') {
          setIsDragOver(false);
          return;
        }
        if (payload.type !== 'drop') return;

        setIsDragOver(false);

        const items: DndResolvedPath[] = [];
        for (const path of payload.paths) {
          const blessed = await blessDroppedPath(path);
          if (blessed.path) {
            items.push(blessed);
          } else {
            log.warn('Dropped path could not be resolved', { path });
          }
        }

        if (items.length === 0) {
          log.warn('Drop produced no resolvable paths', { count: payload.paths.length });
          return;
        }

        await onDropRef.current(items);
      })
      .then((off) => {
        if (disposed) off();
        else unlisten = off;
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [disabled, log]);

  const handleDragEnter = useCallback((e: DragEvent) => {
    e.preventDefault();
  }, []);

  const handleDragOver = useCallback((e: DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'copy';
  }, []);

  const handleDragLeave = useCallback((e: DragEvent) => {
    e.preventDefault();
  }, []);

  const handleDrop = useCallback((e: DragEvent) => {
    e.preventDefault();
  }, []);

  return {
    isDragOver,
    bind: {
      onDragEnter: handleDragEnter,
      onDragOver: handleDragOver,
      onDragLeave: handleDragLeave,
      onDrop: handleDrop,
    },
  };
}
