import { invoke } from '@tauri-apps/api/core';
import type { Project, Artifact, ArtifactData, Session } from '../types';

// =============================================================================
// Project operations
// =============================================================================

/**
 * Create a new project.
 */
export async function createProject(name: string): Promise<Project> {
  return invoke<Project>('create_project', { name });
}

/**
 * Get a project by ID.
 */
export async function getProject(projectId: string): Promise<Project | null> {
  return invoke<Project | null>('get_project', { projectId });
}

/**
 * List all projects.
 */
export async function listProjects(): Promise<Project[]> {
  return invoke<Project[]>('list_projects');
}

/**
 * Update a project's name.
 */
export async function updateProject(projectId: string, name: string): Promise<void> {
  return invoke('update_project', { projectId, name });
}

/**
 * Delete a project and all its artifacts.
 */
export async function deleteProject(projectId: string): Promise<void> {
  return invoke('delete_project', { projectId });
}

// =============================================================================
// Artifact operations
// =============================================================================

/**
 * Create a new artifact.
 */
export async function createArtifact(
  projectId: string,
  title: string,
  data: ArtifactData
): Promise<Artifact> {
  return invoke<Artifact>('create_artifact', { projectId, title, data });
}

/**
 * Get an artifact by ID.
 */
export async function getArtifact(artifactId: string): Promise<Artifact | null> {
  return invoke<Artifact | null>('get_artifact', { artifactId });
}

/**
 * List artifacts in a project.
 */
export async function listArtifacts(projectId: string): Promise<Artifact[]> {
  return invoke<Artifact[]>('list_artifacts', { projectId });
}

/**
 * Update an artifact's title and/or data.
 */
export async function updateArtifact(
  artifactId: string,
  title?: string,
  data?: ArtifactData
): Promise<void> {
  return invoke('update_artifact', { artifactId, title, data });
}

/**
 * Delete an artifact.
 */
export async function deleteArtifact(artifactId: string): Promise<void> {
  return invoke('delete_project_artifact', { artifactId });
}

// =============================================================================
// Context operations
// =============================================================================

/**
 * Add context links to an artifact (which artifacts were used as input).
 */
export async function addArtifactContext(
  artifactId: string,
  contextArtifactIds: string[]
): Promise<void> {
  return invoke('add_artifact_context', { artifactId, contextArtifactIds });
}

/**
 * Get the artifacts that were used as context when creating an artifact.
 */
export async function getArtifactContext(artifactId: string): Promise<string[]> {
  return invoke<string[]>('get_artifact_context', { artifactId });
}

// =============================================================================
// Session operations
// =============================================================================

/**
 * Save a session (AI conversation transcript) for an artifact.
 */
export async function saveSession(artifactId: string, transcript: string): Promise<Session> {
  return invoke<Session>('save_session', { artifactId, transcript });
}

/**
 * Get sessions for an artifact.
 */
export async function getSessions(artifactId: string): Promise<Session[]> {
  return invoke<Session[]>('get_sessions', { artifactId });
}
