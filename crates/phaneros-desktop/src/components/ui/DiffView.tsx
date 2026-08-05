import React from 'react';
import { CodeDiff, BinaryMetadataDiff } from '@/types';

export interface DiffViewProps {
  textDiff?: CodeDiff;
  binaryDiff?: BinaryMetadataDiff;
  onKeepLocal?: () => void;
  onKeepServer?: () => void;
}

export const DiffView: React.FC<DiffViewProps> = ({
  textDiff,
  binaryDiff,
  onKeepLocal,
  onKeepServer,
}) => {
  return (
    <div className="flex flex-col gap-4 w-full font-sans">
      {/* Toolbar actions */}
      <div className="flex items-center justify-between p-3 bg-muted rounded-lg border border-border">
        <span className="font-mono text-sm font-semibold">
          {textDiff?.filename || binaryDiff?.filename || 'Conflict Workspace'}
        </span>
        <div className="flex gap-2">
          <button
            type="button"
            onClick={onKeepLocal}
            className="px-3 py-1.5 text-xs font-semibold rounded bg-primary text-primary-foreground hover:opacity-90"
          >
            Keep Local
          </button>
          <button
            type="button"
            onClick={onKeepServer}
            className="px-3 py-1.5 text-xs font-semibold rounded border border-border bg-card text-foreground hover:bg-accent"
          >
            Keep Server
          </button>
        </div>
      </div>

      {/* Binary Metadata Matrix */}
      {binaryDiff && (
        <div className="border border-border rounded-xl p-4 bg-card" data-testid="binary-matrix">
          <h3 className="text-sm font-serif font-bold mb-3">Binary File Comparison Matrix</h3>
          <table className="w-full text-xs text-left border-collapse">
            <thead>
              <tr className="border-b border-border bg-muted/50">
                <th className="p-2">Property</th>
                <th className="p-2">Local File</th>
                <th className="p-2">Store / Server File</th>
              </tr>
            </thead>
            <tbody>
              <tr className="border-b border-border">
                <td className="p-2 font-semibold">Size</td>
                <td className="p-2 font-mono">{binaryDiff.local.size}</td>
                <td className="p-2 font-mono">{binaryDiff.store.size}</td>
              </tr>
              <tr className="border-b border-border">
                <td className="p-2 font-semibold">Modified</td>
                <td className="p-2 font-mono">{binaryDiff.local.modified}</td>
                <td className="p-2 font-mono">{binaryDiff.store.modified}</td>
              </tr>
              <tr className="border-b border-border">
                <td className="p-2 font-semibold">SHA-256 Hash</td>
                <td className="p-2 font-mono text-[10px] break-all">{binaryDiff.local.hash}</td>
                <td className="p-2 font-mono text-[10px] break-all">{binaryDiff.store.hash}</td>
              </tr>
              <tr>
                <td className="p-2 font-semibold">Recommended Action</td>
                <td colSpan={2} className="p-2 font-semibold text-primary">
                  {binaryDiff.recommendedAction}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      )}

      {/* Side-by-Side Text Diffing */}
      {textDiff && (
        <div className="border border-border rounded-xl bg-card overflow-hidden" data-testid="side-by-side-diff">
          <div className="flex border-b border-border bg-muted/50 text-xs font-mono font-semibold">
            <div className="w-1/2 p-2 border-r border-border">Local Version</div>
            <div className="w-1/2 p-2">Server Version</div>
          </div>
          {textDiff.chunks.map((chunk, idx) => (
            <div key={idx} className="divide-y divide-border">
              {chunk.lines.map((line, lineIdx) => {
                const renderWordHighlights = (type: 'add' | 'delete') => {
                  if (!line.wordHighlights) return line.text;
                  const highlightClass = type === 'add' ? 'diffs-word-added' : 'diffs-word-removed';
                  return line.text.split(' ').map((word, wIdx) => {
                    const cleanWord = word.replace(/^[+-]\s*/, '');
                    const match = cleanWord.length > 0 && line.wordHighlights?.some((wh) => wh.word === cleanWord);
                    return match ? (
                      <span key={wIdx} className={highlightClass}>
                        {cleanWord}{' '}
                      </span>
                    ) : (
                      word + ' '
                    );
                  });
                };

                return (
                  <div key={lineIdx} className="flex text-xs font-mono leading-relaxed">
                    <div
                      className={`w-1/2 p-2 border-r border-border ${
                        line.type === 'delete' ? 'bg-red-500/10 text-red-700 dark:text-red-300' : ''
                      }`}
                    >
                      {line.type === 'delete' ? renderWordHighlights('delete') : line.type === 'same' ? line.text : ''}
                    </div>
                    <div
                      className={`w-1/2 p-2 ${
                        line.type === 'add' ? 'bg-green-500/10 text-green-700 dark:text-green-300' : ''
                      }`}
                    >
                      {line.type === 'add' ? renderWordHighlights('add') : line.type === 'same' ? line.text : ''}
                    </div>
                  </div>
                );
              })}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
