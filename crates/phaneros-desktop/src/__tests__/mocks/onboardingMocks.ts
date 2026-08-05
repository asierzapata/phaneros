import { OnboardingState } from '@/types';
import { mockDrives } from './vaultMocks';

const baseAsyncState = {
  isTestingConnection: false,
  connectionError: null,
  isCompleting: false,
  completionError: null,
};

export const mockOnboardingStep1: OnboardingState = {
  currentStep: 1,
  isCompleted: false,
  destinationMode: 'cloud',
  serverUrl: '',
  serverToken: '',
  isConnected: false,
  vaults: [],
  ...baseAsyncState,
};

export const mockOnboardingStep2CloudConnected: OnboardingState = {
  currentStep: 2,
  isCompleted: false,
  destinationMode: 'cloud',
  serverUrl: '',
  serverToken: '',
  isConnected: true,
  vaults: [],
  ...baseAsyncState,
};

export const mockOnboardingStep2SelfHosted: OnboardingState = {
  currentStep: 2,
  isCompleted: false,
  destinationMode: 'self-hosted',
  serverUrl: 'https://sync.mycompany.com:8443',
  serverToken: 'sec_token_987654321',
  isConnected: true,
  vaults: [],
  ...baseAsyncState,
};

export const mockOnboardingStep3VaultsSelected: OnboardingState = {
  currentStep: 3,
  isCompleted: false,
  destinationMode: 'cloud',
  serverUrl: '',
  serverToken: '',
  isConnected: true,
  vaults: mockDrives,
  ...baseAsyncState,
};

export const mockOnboardingStep4TestDrive: OnboardingState = {
  currentStep: 4,
  isCompleted: false,
  destinationMode: 'cloud',
  serverUrl: '',
  serverToken: '',
  isConnected: true,
  vaults: mockDrives,
  ...baseAsyncState,
};

export const mockOnboardingStep5Completed: OnboardingState = {
  currentStep: 5,
  isCompleted: true,
  destinationMode: 'cloud',
  serverUrl: '',
  serverToken: '',
  isConnected: true,
  vaults: mockDrives,
  ...baseAsyncState,
};
