export type StartupPolicy = "auto" | "ask" | "manual";

export type ProjectStatus =
  | "running"
  | "stopped"
  | "starting"
  | "stopping"
  | "crashed"
  | "restorePending";

export interface AppSettings {
  onboardingComplete: boolean;
  launchAtLogin: boolean;
  defaultStartupPolicy: StartupPolicy;
  discoveryEnabled: boolean;
  autoRegisterDiscovered: boolean;
  workspaceRoots: string[];
}

export type ProcessOrigin = "manager" | "external";

export interface Project {
  id: string;
  name: string;
  directory: string;
  command: string;
  url?: string;
  startupPolicy: StartupPolicy;
  status: ProjectStatus;
  desiredRunning: boolean;
  pid?: number;
  createdAt: number;
  updatedAt: number;
  lastStartedAt?: number;
  lastStoppedAt?: number;
  lastExitCode?: number;
  lastError?: string;
  logPath: string;
  discoveryKey?: string;
  detectedPort?: number;
  processOrigin: ProcessOrigin;
}

export interface DiscoveryCandidate {
  key: string;
  pid: number;
  port: number;
  address: string;
  url: string;
  name: string;
  processName: string;
  processType: string;
  executable: string;
  command: string;
  directory: string;
  externalExposure: boolean;
  confidence: number;
  discoveredAt: number;
}

export interface DashboardSnapshot {
  settings: AppSettings;
  projects: Project[];
  restoreCount: number;
  runningCount: number;
  crashedCount: number;
  discoveryCandidates: DiscoveryCandidate[];
  discoveryCount: number;
  ignoredDiscoveryCount: number;
}

export interface ProjectInput {
  name: string;
  directory: string;
  command: string;
  url?: string;
  startupPolicy: StartupPolicy;
  discoveryKey?: string;
  detectedPort?: number;
  externalPid?: number;
}

export interface SettingsInput {
  onboardingComplete: boolean;
  launchAtLogin: boolean;
  defaultStartupPolicy: StartupPolicy;
  discoveryEnabled: boolean;
  autoRegisterDiscovered: boolean;
  workspaceRoots: string[];
}
