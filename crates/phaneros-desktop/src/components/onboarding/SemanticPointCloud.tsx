import React, { useEffect, useRef } from 'react';
import { useTheme } from '@/context/ThemeContext';

interface Point {
  x: number;
  y: number;
  z: number;
  size: number;
  colorType: 'default' | 'accent' | 'green';
}

export interface SemanticPointCloudProps {
  step: number;
  isTesting?: boolean;
  folderCount?: number;
}

/**
 * Rotating 3D point-cloud "semantic art" for the onboarding wizard's right
 * column, ported from documentation/reference/design-mockups/phaneros_onboarding.html
 * (generateSemanticPoints/renderSemanticCanvas). Each step gets its own
 * hand-placed point geometry (disk/lock, globe, folder scanner, heightmap,
 * infinity vault) rotated and perspective-projected onto the 2D canvas.
 */
export const SemanticPointCloud: React.FC<SemanticPointCloudProps> = ({
  step,
  isTesting = false,
  folderCount = 2,
}) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const { theme } = useTheme();

  const stepRef = useRef(step);
  const isTestingRef = useRef(isTesting);
  const folderCountRef = useRef(folderCount);
  const themeRef = useRef(theme);
  stepRef.current = step;
  isTestingRef.current = isTesting;
  folderCountRef.current = folderCount;
  themeRef.current = theme;

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    let width = 0;
    let height = 0;
    let rotAngle = 0;
    let frameId = 0;

    const resizeCanvas = () => {
      const rect = canvas.parentElement?.getBoundingClientRect();
      if (!rect) return;
      canvas.width = rect.width;
      canvas.height = rect.height;
      width = canvas.width;
      height = canvas.height;
    };

    const generateSemanticPoints = (currentStep: number): Point[] => {
      const pts: Point[] = [];
      const testing = isTestingRef.current;

      if (currentStep === 1) {
        // Disk platter rings + floating encrypted lock vault.
        [45, 65].forEach((yPos, layerIdx) => {
          for (let r = 15; r <= 110; r += 16) {
            const dotsOnRing = Math.max(6, Math.floor(r / 2.2));
            for (let i = 0; i < dotsOnRing; i++) {
              const a = (i / dotsOnRing) * Math.PI * 2 + (layerIdx === 0 ? rotAngle * 0.5 : -rotAngle * 0.5);
              pts.push({
                x: Math.cos(a) * r,
                y: yPos + Math.sin(a * 3) * 2,
                z: Math.sin(a) * r,
                size: r === 110 || r === 15 ? 2.0 : 1.4,
                colorType: 'default',
              });
            }
          }
        });

        for (let t = 0; t <= 1; t += 0.08) {
          pts.push({ x: -90 + t * 75, y: 35, z: -50 + t * 45, size: 2.2, colorType: 'accent' });
        }

        const lockYOffset = -45 + Math.sin(rotAngle * 2) * 6;
        for (let a = Math.PI; a <= Math.PI * 2; a += 0.25) {
          pts.push({ x: Math.cos(a) * 28, y: lockYOffset - 30 + Math.sin(a) * 22, z: Math.sin(a) * 12, size: 2.4, colorType: 'accent' });
          pts.push({ x: Math.cos(a) * 28, y: lockYOffset - 30 + Math.sin(a) * 22, z: -Math.sin(a) * 12, size: 2.4, colorType: 'accent' });
        }
        for (let bx = -32; bx <= 32; bx += 10) {
          for (let by = -20; by <= 20; by += 10) {
            for (let bz = -14; bz <= 14; bz += 14) {
              if (Math.abs(bx) === 32 || Math.abs(by) === 20 || Math.abs(bz) === 14) {
                pts.push({ x: bx, y: lockYOffset + by, z: bz, size: 2.0, colorType: 'accent' });
              }
            }
          }
        }
        pts.push({ x: 0, y: lockYOffset, z: 16, size: 4.0, colorType: 'green' });
      } else if (currentStep === 2) {
        // Halftone globe + satellite destination nodes.
        const radius = 120;
        for (let lat = -80; lat <= 80; lat += 10) {
          const r = radius * Math.cos((lat * Math.PI) / 180);
          const y = radius * Math.sin((lat * Math.PI) / 180);
          for (let lon = 0; lon < 360; lon += 14) {
            pts.push({ x: r * Math.cos((lon * Math.PI) / 180), y, z: r * Math.sin((lon * Math.PI) / 180), size: 1.8, colorType: 'default' });
          }
        }
        for (let a = 0; a < Math.PI * 2; a += 0.2) {
          const isNode = Math.floor(a * 10) % 12 === 0;
          pts.push({ x: Math.cos(a) * 155, y: Math.sin(a) * 45, z: Math.sin(a) * 155, size: isNode ? 3.5 : 2.0, colorType: isNode ? 'green' : 'accent' });
        }
      } else if (currentStep === 3) {
        // Folder scanner network + dynamic file streams.
        const folders = Math.max(1, folderCountRef.current);
        for (let idx = 0; idx < folders; idx++) {
          const zOffset = -50 + idx * 35;
          const yOffset = -35 + idx * 25;
          const xOffset = -20 + (idx % 2 === 0 ? -10 : 15);
          const fWidth = 100;
          const fHeight = 60;
          for (let x = -fWidth / 2; x <= fWidth / 2; x += 10) {
            for (let y = -fHeight / 2; y <= fHeight / 2; y += 10) {
              const isTab = y < -fHeight / 2 + 10 && x < -fWidth / 6;
              const isEdge = Math.abs(x) >= fWidth / 2 - 5 || Math.abs(y) >= fHeight / 2 - 5 || isTab;
              if (isEdge) {
                pts.push({ x: x + xOffset, y: y + yOffset, z: zOffset, size: 2.0, colorType: 'default' });
              }
            }
          }
        }

        const scanY = Math.sin(rotAngle * 2.2) * 85;
        for (let a = 0; a < Math.PI * 2; a += 0.18) {
          pts.push({ x: Math.cos(a) * 125, y: scanY, z: Math.sin(a) * 80, size: 2.5, colorType: 'green' });
        }

        for (let i = 0; i < 24; i++) {
          const streamPhase = (rotAngle * 40 + i * 15) % 160;
          pts.push({ x: Math.sin(i * 1.5) * 40, y: 80 - streamPhase, z: Math.cos(i * 1.5) * 40, size: 2.2, colorType: 'accent' });
        }
      } else if (currentStep === 4) {
        // Topographic heightmap with a test-drive ripple surge.
        const range = 120;
        const stepSize = 14;
        for (let x = -range; x <= range; x += stepSize) {
          for (let z = -range; z <= range; z += stepSize) {
            const dist = Math.sqrt(x * x + z * z);
            const waveAmp = testing ? 48 : 24;
            const y = Math.sin(dist * 0.06 - rotAngle * 3.5) * waveAmp;
            pts.push({ x, y, z, size: dist < 40 && testing ? 3.0 : 1.8, colorType: testing ? 'green' : 'default' });
          }
        }
      } else if (currentStep === 5) {
        // Infinity vault + harmonious orbit rings.
        for (let a = 0; a <= Math.PI * 2; a += 0.3) {
          pts.push({ x: Math.cos(a) * 36, y: Math.sin(a) * 36, z: Math.sin(a * 2) * 10, size: 2.2, colorType: 'green' });
        }
        pts.push({ x: 0, y: 0, z: 20, size: 4.5, colorType: 'green' });

        for (let t = 0; t < Math.PI * 2; t += 0.08) {
          const denom = 1 + Math.sin(t) * Math.sin(t);
          pts.push({ x: (140 * Math.cos(t)) / denom, y: Math.sin(t * 2) * 22, z: (140 * Math.sin(t) * Math.cos(t)) / denom, size: 1.8, colorType: 'default' });
        }

        for (let i = 0; i < 16; i++) {
          const loopT = (rotAngle * 2 + (i * Math.PI * 2) / 16) % (Math.PI * 2);
          const denom = 1 + Math.sin(loopT) * Math.sin(loopT);
          pts.push({ x: (140 * Math.cos(loopT)) / denom, y: Math.sin(loopT * 2) * 22, z: (140 * Math.sin(loopT) * Math.cos(loopT)) / denom, size: 3.2, colorType: 'green' });
        }

        for (let a = 0; a < Math.PI * 2; a += 0.15) {
          pts.push({ x: Math.cos(a) * 165, y: Math.sin(a * 3) * 8, z: Math.sin(a) * 165, size: 1.4, colorType: 'accent' });
        }
      }

      return pts;
    };

    const render = () => {
      ctx.clearRect(0, 0, width, height);
      const cx = width / 2;
      const cy = height / 2;
      rotAngle += 0.0025;

      const isDark = themeRef.current === 'dark';
      const baseBlue = isDark ? 'rgba(96, 165, 250, ' : 'rgba(37, 99, 235, ';
      const greenAccent = isDark ? 'rgba(52, 211, 153, ' : 'rgba(5, 150, 105, ';
      const purpleAccent = isDark ? 'rgba(192, 132, 252, ' : 'rgba(147, 51, 234, ';

      const currentStep = stepRef.current;
      const points = generateSemanticPoints(currentStep);
      const cos = Math.cos(rotAngle);
      const sin = Math.sin(rotAngle);

      points.forEach((p) => {
        const rx = p.x * cos - p.z * sin;
        const rz = p.x * sin + p.z * cos;
        const ry = p.y;

        const scale = 320 / (320 + rz);
        const screenX = cx + rx * scale;
        const screenY = cy + ry * scale;
        const alpha = Math.max(0.18, Math.min(0.95, (rz + 160) / 320));

        let dotColor = baseBlue;
        if (p.colorType === 'green' || isTestingRef.current || currentStep === 5) {
          dotColor = greenAccent;
        } else if (p.colorType === 'accent') {
          dotColor = purpleAccent;
        }

        ctx.beginPath();
        ctx.arc(screenX, screenY, p.size * scale, 0, Math.PI * 2);
        ctx.fillStyle = dotColor + alpha + ')';
        ctx.fill();
      });

      frameId = requestAnimationFrame(render);
    };

    resizeCanvas();
    window.addEventListener('resize', resizeCanvas);
    frameId = requestAnimationFrame(render);

    return () => {
      window.removeEventListener('resize', resizeCanvas);
      cancelAnimationFrame(frameId);
    };
  }, []);

  return <canvas ref={canvasRef} data-testid="onboarding-point-canvas" className="block w-full h-full" />;
};
