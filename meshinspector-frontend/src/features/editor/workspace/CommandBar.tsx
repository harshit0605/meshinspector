'use client';

import { useEffect, useRef, useState } from 'react';
import { COMMANDS_BY_GROUP, TOOLBAR_GROUP_LABELS, TOOLBAR_GROUP_ORDER } from './toolRegistry';
import type { ContextToolId, ToolbarGroup, WorkspaceCommandId } from './types';

type CommandAvailability = {
  disabled: boolean;
  reason?: string;
};

export default function CommandBar({
  activeTool,
  openPopoverGroup,
  onGroupOpen,
  onGroupClose,
  onCommandSelect,
  getCommandAvailability,
}: {
  activeTool: ContextToolId | null;
  openPopoverGroup: ToolbarGroup | null;
  onGroupOpen: (group: ToolbarGroup) => void;
  onGroupClose: () => void;
  onCommandSelect: (commandId: WorkspaceCommandId) => void;
  getCommandAvailability: (commandId: WorkspaceCommandId) => CommandAvailability;
}) {
  const shellRef = useRef<HTMLDivElement | null>(null);
  const rowRef = useRef<HTMLDivElement | null>(null);
  const closeTimerRef = useRef<number | null>(null);
  const buttonRefs = useRef<Record<ToolbarGroup, HTMLButtonElement | null>>({
    file: null,
    prepare: null,
    modify: null,
    inspect: null,
    review: null,
  });
  const [popoverStyle, setPopoverStyle] = useState<{ left: number; width: number } | null>(null);

  const clearCloseTimer = () => {
    if (closeTimerRef.current != null) {
      window.clearTimeout(closeTimerRef.current);
      closeTimerRef.current = null;
    }
  };

  const scheduleClose = () => {
    clearCloseTimer();
    closeTimerRef.current = window.setTimeout(() => {
      onGroupClose();
      closeTimerRef.current = null;
    }, 140);
  };

  useEffect(() => {
    function handlePointerDown(event: MouseEvent) {
      if (!shellRef.current?.contains(event.target as Node)) {
        clearCloseTimer();
        onGroupClose();
      }
    }

    if (!openPopoverGroup) {
      return;
    }

    document.addEventListener('mousedown', handlePointerDown);
    return () => document.removeEventListener('mousedown', handlePointerDown);
  }, [onGroupClose, openPopoverGroup]);

  useEffect(() => {
    return () => {
      clearCloseTimer();
    };
  }, []);

  useEffect(() => {
    function updatePopoverPosition() {
      if (!openPopoverGroup || !shellRef.current || !buttonRefs.current[openPopoverGroup]) {
        setPopoverStyle(null);
        return;
      }

      const shellRect = shellRef.current.getBoundingClientRect();
      const buttonRect = buttonRefs.current[openPopoverGroup]!.getBoundingClientRect();
      const width = Math.min(540, Math.max(shellRect.width - 16, 280));
      const rawLeft = buttonRect.left - shellRect.left;
      const maxLeft = Math.max(shellRect.width - width, 0);
      setPopoverStyle({
        left: Math.min(Math.max(rawLeft, 0), maxLeft),
        width,
      });
    }

    updatePopoverPosition();

    if (!openPopoverGroup) {
      return;
    }

    const row = rowRef.current;
    window.addEventListener('resize', updatePopoverPosition);
    row?.addEventListener('scroll', updatePopoverPosition, { passive: true });
    return () => {
      window.removeEventListener('resize', updatePopoverPosition);
      row?.removeEventListener('scroll', updatePopoverPosition);
    };
  }, [openPopoverGroup]);

  const openGroupCommands = openPopoverGroup ? COMMANDS_BY_GROUP[openPopoverGroup] : [];

  return (
    <div
      ref={shellRef}
      onMouseEnter={clearCloseTimer}
      onMouseLeave={scheduleClose}
      className="relative z-30 border-b border-zinc-800 bg-zinc-950/90 px-4 py-2 backdrop-blur"
    >
      <div ref={rowRef} className="flex items-center gap-1 overflow-x-auto">
        {TOOLBAR_GROUP_ORDER.map((group) => {
          const hasActiveTool = COMMANDS_BY_GROUP[group].some((command) => command.contextualToolId === activeTool);
          return (
            <div
              key={group}
              className="shrink-0"
              onMouseEnter={() => {
                clearCloseTimer();
                onGroupOpen(group);
              }}
            >
              <button
                ref={(node) => {
                  buttonRefs.current[group] = node;
                }}
                onFocus={() => onGroupOpen(group)}
                className={`rounded-lg px-3 py-2 text-sm transition-colors ${
                  openPopoverGroup === group || hasActiveTool
                    ? 'bg-zinc-100 text-zinc-950'
                    : 'text-zinc-300 hover:bg-zinc-900 hover:text-white'
                }`}
              >
                {TOOLBAR_GROUP_LABELS[group]}
              </button>
            </div>
          );
        })}
      </div>

      {openPopoverGroup && popoverStyle ? (
        <div
          onMouseEnter={clearCloseTimer}
          onMouseLeave={scheduleClose}
          className="absolute top-[calc(100%+10px)] z-40 rounded-2xl border border-zinc-800 bg-zinc-950/98 p-3 shadow-[0_24px_90px_rgba(0,0,0,0.5)] backdrop-blur"
          style={popoverStyle}
        >
          <div className="grid gap-2 sm:grid-cols-2">
            {openGroupCommands.map((command) => {
              const availability = getCommandAvailability(command.id);
              const selected = activeTool === command.contextualToolId;
              return (
                <button
                  key={command.id}
                  onClick={() => onCommandSelect(command.id)}
                  disabled={availability.disabled}
                  className={`rounded-xl border p-3 text-left transition-colors ${
                    selected
                      ? 'border-amber-500/40 bg-amber-500/12'
                      : 'border-zinc-800 bg-zinc-900/80 hover:bg-zinc-900'
                  } disabled:cursor-not-allowed disabled:opacity-45`}
                >
                  <div className="flex items-start gap-3">
                    <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border border-zinc-700 bg-zinc-950 text-[11px] font-semibold text-zinc-200">
                      {command.icon}
                    </div>
                    <div className="min-w-0">
                      <p className="text-sm font-medium text-zinc-100">{command.label}</p>
                      <p className="mt-1 text-xs leading-5 text-zinc-500">{command.description}</p>
                      {availability.reason ? (
                        <p className="mt-2 text-[11px] text-amber-300">{availability.reason}</p>
                      ) : null}
                    </div>
                  </div>
                </button>
              );
            })}
          </div>
        </div>
      ) : null}
    </div>
  );
}
