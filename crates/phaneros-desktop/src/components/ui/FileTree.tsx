import React, { useState } from 'react';
import { FileNode } from '@/types';
import { Badge } from './Badge';

export interface FileTreeProps {
  nodes: FileNode[];
  onSelectNode?: (node: FileNode) => void;
}

const FileTreeNode: React.FC<{
  node: FileNode;
  level: number;
  onSelectNode?: (node: FileNode) => void;
}> = ({ node, level, onSelectNode }) => {
  const [isExpanded, setIsExpanded] = useState(true);

  const toggleExpand = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (node.isDir) {
      setIsExpanded(!isExpanded);
    }
  };

  const handleSelect = () => {
    if (onSelectNode) {
      onSelectNode(node);
    }
  };

  return (
    <div className="font-sans text-xs select-none">
      <div
        onClick={handleSelect}
        className="flex items-center gap-2 py-1 px-2 hover:bg-accent/50 rounded cursor-pointer group"
        style={{ paddingLeft: `${level * 16 + 8}px` }}
        data-testid={`file-node-${node.id}`}
      >
        {node.isDir ? (
          <button
            type="button"
            onClick={toggleExpand}
            aria-label={isExpanded ? 'Collapse folder' : 'Expand folder'}
            className="w-4 h-4 flex items-center justify-center text-muted-foreground hover:text-foreground font-mono font-bold"
          >
            {isExpanded ? '▼' : '►'}
          </button>
        ) : (
          <span className="w-4 h-4 inline-block text-center text-muted-foreground font-mono">📄</span>
        )}

        <span className="font-medium text-foreground">{node.name}</span>

        {node.badge && (
          <Badge variant="mono" className="font-mono text-[10px] uppercase">
            {node.badge}
          </Badge>
        )}

        {node.size && <span className="ml-auto text-muted-foreground text-[10px] font-mono">{node.size}</span>}
      </div>

      {node.isDir && isExpanded && node.children && (
        <div className="tree-indent-guide border-l ml-3">
          {node.children.map((child) => (
            <FileTreeNode
              key={child.id}
              node={child}
              level={level + 1}
              onSelectNode={onSelectNode}
            />
          ))}
        </div>
      )}
    </div>
  );
};

export const FileTree: React.FC<FileTreeProps> = ({ nodes, onSelectNode }) => {
  return (
    <div className="w-full flex flex-col gap-1" data-testid="interactive-file-tree">
      {nodes.map((node) => (
        <FileTreeNode key={node.id} node={node} level={0} onSelectNode={onSelectNode} />
      ))}
    </div>
  );
};
