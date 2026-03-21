'use client';

import { type ToolType } from './Navbar';

const TOOL_DEFINITIONS: Record<Exclude<ToolType, 'none' | 'info'>, { icon: string; label: string; color: string }> = {
  section: { icon: 'Section', label: 'Section', color: 'amber' },
  hollow: { icon: 'Hollow', label: 'Hollow', color: 'amber' },
  measure: { icon: 'Measure', label: 'Measure', color: 'blue' },
  repair: { icon: 'Repair', label: 'Repair', color: 'emerald' },
  inspect: { icon: 'Inspect', label: 'Inspect', color: 'sky' },
};

interface ToolSidebarProps {
  activeTools: Set<ToolType>;
  focusedTool: ToolType;
  onToolClick: (tool: ToolType) => void;
  onToolRemove: (tool: ToolType) => void;
}

export default function ToolSidebar({
  activeTools,
  focusedTool,
  onToolClick,
  onToolRemove,
}: ToolSidebarProps) {
  const visibleTools = Array.from(activeTools).filter(
    (tool): tool is Exclude<ToolType, 'none' | 'info'> => tool !== 'none' && tool !== 'info' && tool in TOOL_DEFINITIONS,
  );

  if (visibleTools.length === 0) {
    return null;
  }

  return (
    <div className="w-14 shrink-0 h-full bg-zinc-900/95 border-l border-zinc-800 flex flex-col items-center py-2 gap-2">
      {visibleTools.map((tool) => {
        const def = TOOL_DEFINITIONS[tool];
        const isFocused = focusedTool === tool;
        return (
          <div key={tool} className="relative group w-11">
            <button
              onClick={() => onToolClick(tool)}
              className={`w-11 h-11 rounded-xl border text-[10px] leading-tight px-1 transition-colors ${
                isFocused
                  ? 'border-amber-500/40 bg-amber-500/15 text-amber-200'
                  : 'border-zinc-800 bg-zinc-950 text-zinc-300 hover:bg-zinc-900'
              }`}
              title={def.label}
            >
              {def.icon}
            </button>
            <button
              onClick={(event) => {
                event.stopPropagation();
                onToolRemove(tool);
              }}
              className="absolute -top-1 -right-1 w-4 h-4 rounded-full bg-zinc-700 text-[10px] text-zinc-200 opacity-0 group-hover:opacity-100"
              title={`Remove ${def.label}`}
            >
              x
            </button>
          </div>
        );
      })}
    </div>
  );
}
