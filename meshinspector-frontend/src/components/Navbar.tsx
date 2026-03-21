'use client';

/**
 * Modern navbar with dropdown menus and active overlay indicators
 */

import { useState, useRef, useEffect } from 'react';

export type ToolType =
    | 'none'
    | 'section'
    | 'hollow'
    | 'measure'
    | 'repair'
    | 'inspect'
    | 'info';

// Overlay definitions for extensibility
export const OVERLAYS = {
    section: { icon: '✂️', label: 'Section', color: 'amber' },
    wireframe: { icon: '⊞', label: 'Wireframe', color: 'blue' },
} as const;

export type OverlayType = keyof typeof OVERLAYS;

interface NavbarProps {
    activeTool: ToolType;
    onToolChange: (tool: ToolType) => void;
    onUpload: () => void;
    onDownload: (format: 'glb' | 'stl') => void;
    onWireframeToggle: () => void;
    onResetCamera: () => void;
    wireframe: boolean;
    modelLoaded: boolean;
    // Active overlays
    sectionEnabled?: boolean;
    onSectionToggle?: () => void;
}

interface MenuItem {
    label: string;
    icon?: string;
    action?: () => void;
    tool?: ToolType;
    divider?: boolean;
    disabled?: boolean;
}

interface MenuCategory {
    label: string;
    items: MenuItem[];
}

export default function Navbar({
    activeTool,
    onToolChange,
    onUpload,
    onDownload,
    onWireframeToggle,
    onResetCamera,
    wireframe,
    modelLoaded,
    sectionEnabled = false,
    onSectionToggle,
}: NavbarProps) {
    const [openMenu, setOpenMenu] = useState<string | null>(null);
    const navRef = useRef<HTMLElement>(null);

    // Close menu on outside click
    useEffect(() => {
        function handleClickOutside(event: MouseEvent) {
            if (navRef.current && !navRef.current.contains(event.target as Node)) {
                setOpenMenu(null);
            }
        }
        document.addEventListener('mousedown', handleClickOutside);
        return () => document.removeEventListener('mousedown', handleClickOutside);
    }, []);

    const menus: MenuCategory[] = [
        {
            label: 'File',
            items: [
                { label: 'Upload Model', icon: '📁', action: onUpload },
                { divider: true, label: '' },
                { label: 'Download GLB', icon: '💾', action: () => onDownload('glb'), disabled: !modelLoaded },
                { label: 'Download STL', icon: '📐', action: () => onDownload('stl'), disabled: !modelLoaded },
            ],
        },
        {
            label: 'View',
            items: [
                { label: wireframe ? '✓ Wireframe' : 'Wireframe', icon: '🔲', action: onWireframeToggle },
                { label: 'Reset Camera', icon: '🎯', action: onResetCamera },
                { divider: true, label: '' },
                { label: 'Model Info', icon: '📊', tool: 'info' },
            ],
        },
        {
            label: 'Inspect',
            items: [
                { label: 'Section', icon: '✂️', tool: 'section' },
                { label: 'Health & Thickness', icon: '🔍', tool: 'inspect' },
                { label: 'Measure', icon: '📏', tool: 'measure', disabled: true },
            ],
        },
        {
            label: 'Mesh',
            items: [
                { label: 'Hollow', icon: '🔘', tool: 'hollow' },
                { label: 'Auto Repair', icon: '🔧', tool: 'repair' },
            ],
        },
    ];

    const handleItemClick = (item: MenuItem) => {
        if (item.disabled) return;

        if (item.action) {
            item.action();
        }
        if (item.tool) {
            onToolChange(item.tool === activeTool ? 'none' : item.tool);
        }
        setOpenMenu(null);
    };

    return (
        <nav ref={navRef} className="h-10 bg-zinc-900/95 border-b border-zinc-800 flex items-center px-2 gap-1 relative z-50">
            {/* Logo */}
            <a href="/" className="flex items-center gap-2 px-3 py-1 hover:bg-zinc-800 rounded-md transition-colors mr-2">
                <div className="w-5 h-5 rounded bg-gradient-to-br from-amber-400 to-amber-600 flex items-center justify-center">
                    <span className="text-black font-bold text-xs">M</span>
                </div>
                <span className="text-sm font-medium text-white hidden sm:block">MeshInspector</span>
            </a>

            {/* Menu items */}
            {menus.map((menu) => (
                <div key={menu.label} className="relative">
                    <button
                        onClick={() => setOpenMenu(openMenu === menu.label ? null : menu.label)}
                        className={`
              px-3 py-1.5 text-sm rounded-md transition-colors
              ${openMenu === menu.label
                                ? 'bg-amber-500/20 text-amber-400'
                                : 'text-zinc-300 hover:text-white hover:bg-zinc-800'}
            `}
                    >
                        {menu.label}
                    </button>

                    {/* Dropdown */}
                    {openMenu === menu.label && (
                        <div className="absolute top-full left-0 mt-1 min-w-44 bg-zinc-900 border border-zinc-700 rounded-lg shadow-xl py-1 z-50">
                            {menu.items.map((item, idx) =>
                                item.divider ? (
                                    <div key={idx} className="h-px bg-zinc-700 my-1" />
                                ) : (
                                    <button
                                        key={idx}
                                        onClick={() => handleItemClick(item)}
                                        disabled={item.disabled}
                                        className={`
                      w-full px-3 py-2 text-sm text-left flex items-center gap-3
                      ${item.disabled
                                                ? 'text-zinc-600 cursor-not-allowed'
                                                : 'text-zinc-300 hover:bg-zinc-800 hover:text-white'}
                      ${item.tool === activeTool ? 'bg-amber-500/10 text-amber-400' : ''}
                    `}
                                    >
                                        <span className="w-5 text-center">{item.icon}</span>
                                        {item.label}
                                    </button>
                                )
                            )}
                        </div>
                    )}
                </div>
            ))}

            {/* Spacer */}
            <div className="flex-1" />

            {/* Active Overlays */}
            <div className="flex items-center gap-2">
                {/* Section overlay chip */}
                {sectionEnabled && (
                    <button
                        onClick={() => {
                            if (onSectionToggle) onSectionToggle();
                        }}
                        className="flex items-center gap-1.5 px-2.5 py-1 bg-amber-500/15 border border-amber-500/30 rounded-full text-amber-400 hover:bg-amber-500/25 transition-colors group"
                        title="Click to disable Section overlay"
                    >
                        <span className="text-sm">✂️</span>
                        <span className="text-xs font-medium">Section</span>
                        <span className="text-amber-400/60 group-hover:text-amber-300 text-sm ml-0.5">×</span>
                    </button>
                )}

                {/* Wireframe overlay chip */}
                {wireframe && (
                    <button
                        onClick={onWireframeToggle}
                        className="flex items-center gap-1.5 px-2.5 py-1 bg-blue-500/15 border border-blue-500/30 rounded-full text-blue-400 hover:bg-blue-500/25 transition-colors group"
                        title="Click to disable Wireframe overlay"
                    >
                        <span className="text-sm">⊞</span>
                        <span className="text-xs font-medium">Wireframe</span>
                        <span className="text-blue-400/60 group-hover:text-blue-300 text-sm ml-0.5">×</span>
                    </button>
                )}
            </div>

            {/* Active panel indicator */}
            {activeTool !== 'none' && activeTool !== 'info' && (
                <div className="flex items-center gap-2 px-3 py-1 bg-zinc-700/50 rounded-md border border-zinc-600/50">
                    <span className="text-xs text-zinc-300 capitalize">{activeTool} Panel</span>
                    <button
                        onClick={() => onToolChange('none')}
                        className="text-zinc-400 hover:text-zinc-200 text-lg leading-none"
                        title="Close panel"
                    >
                        ×
                    </button>
                </div>
            )}

            {/* Status indicator */}
            <div className="flex items-center gap-2 px-3">
                <div className={`w-2 h-2 rounded-full ${modelLoaded ? 'bg-green-500' : 'bg-zinc-500'}`} />
                <span className="text-xs text-zinc-500 hidden md:block">
                    {modelLoaded ? 'Model Loaded' : 'No Model'}
                </span>
            </div>
        </nav>
    );
}
