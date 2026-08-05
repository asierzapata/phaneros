import React, { createContext, useContext, useEffect, useState } from 'react';
import { DriveVault } from '@/types';
import { mockDrives } from '@/__tests__/mocks/vaultMocks';
import { fetchVaults } from '@/lib/backendBridge';

export interface VaultContextType {
  drives: DriveVault[];
  activeDriveId: string;
  activeDrive: DriveVault | undefined;
  isLoading: boolean;
  error: string | null;
  selectDrive: (id: string) => void;
  addVault: (vault: Partial<DriveVault>) => void;
  removeVault: (id: string) => void;
}

const VaultContext = createContext<VaultContextType | undefined>(undefined);

export interface VaultProviderProps {
  children: React.ReactNode;
  initialDrives?: DriveVault[];
  initialActiveId?: string;
}

export const VaultProvider: React.FC<VaultProviderProps> = ({
  children,
  initialDrives = mockDrives,
  initialActiveId = mockDrives[0]?.id || 'vault-default',
}) => {
  const [drives, setDrives] = useState<DriveVault[]>(initialDrives);
  const [activeDriveId, setActiveDriveId] = useState<string>(initialActiveId);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setIsLoading(true);
    fetchVaults()
      .then((fetched) => {
        if (cancelled || fetched === null) return;
        setDrives(fetched);
        setError(null);
      })
      .catch((err) => {
        if (cancelled) return;
        setError(err instanceof Error ? err.message : String(err));
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const activeDrive = drives.find((d) => d.id === activeDriveId) || drives[0];

  const selectDrive = (id: string) => {
    setActiveDriveId(id);
  };

  const addVault = (vaultPartial: Partial<DriveVault>) => {
    const newVault: DriveVault = {
      id: vaultPartial.id || `vault-${Date.now()}`,
      name: vaultPartial.name ?? 'New Vault',
      path: vaultPartial.path ?? '~/Documents/NewVault',
      status: vaultPartial.status || 'synced',
      usedBytes: vaultPartial.usedBytes || 0,
      quotaBytes: undefined, // Infinite (∞) quota rule
      fileCount: vaultPartial.fileCount || 0,
    };
    setDrives((prev) => [...prev, newVault]);
  };

  const removeVault = (id: string) => {
    setDrives((prev) => prev.filter((d) => d.id !== id));
  };

  return (
    <VaultContext.Provider
      value={{ drives, activeDriveId, activeDrive, isLoading, error, selectDrive, addVault, removeVault }}
    >
      {children}
    </VaultContext.Provider>
  );
};

export const useVault = (): VaultContextType => {
  const context = useContext(VaultContext);
  if (!context) {
    throw new Error('useVault must be used within a VaultProvider');
  }
  return context;
};

export { VaultContext };
