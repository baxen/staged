/**
 * Plugin system for Stage frontend
 *
 * Handles dynamic loading of plugin JavaScript/CSS assets and provides
 * an API for plugins to interact with the Stage application.
 */

import { invoke } from '@tauri-apps/api/core';
import { writable } from 'svelte/store';
import type { DiffSpec } from '../types';

/**
 * Plugin manifest from backend
 */
export interface PluginManifest {
  name: string;
  version: string;
  description?: string;
  frontend?: {
    script: string;
    style?: string;
  };
}

/**
 * API provided to plugins
 *
 * This is the interface that plugin frontend code can use to interact
 * with the Stage application.
 */
export interface PluginAPI {
  /**
   * Invoke a plugin command on the backend
   */
  invoke(command: string, payload: any): Promise<any>;

  /**
   * Register a modal component
   * The modal will be available to trigger from plugin commands
   */
  registerModal(id: string, component: any): void;

  /**
   * Get the current repository path
   */
  getCurrentRepo(): Promise<string | null>;

  /**
   * Get the current diff spec
   */
  getCurrentDiff(): Promise<DiffSpec | null>;

  /**
   * Show a modal registered by this plugin
   */
  showModal(id: string, props?: any): void;

  /**
   * Close the currently open modal
   */
  closeModal(): void;
}

/**
 * Modal state management
 */
interface ModalState {
  component: any | null;
  props: any;
  visible: boolean;
}

/**
 * Plugin system
 *
 * Manages plugin loading, asset injection, and API provision.
 */
class PluginSystem {
  private plugins = new Map<string, PluginManifest>();
  private modals = new Map<string, any>();

  // Use a Svelte store for modal state
  public modalStateStore = writable<ModalState>({
    component: null,
    props: {},
    visible: false,
  });

  /**
   * Initialize the plugin system
   *
   * Discovers and loads all available plugins.
   */
  async initialize(): Promise<void> {
    try {
      const manifests = await invoke<PluginManifest[]>('list_plugins');
      console.log(`Discovered ${manifests.length} plugin(s)`);

      for (const manifest of manifests) {
        try {
          await this.loadPlugin(manifest);
          console.log(`Loaded plugin: ${manifest.name}`);
        } catch (e) {
          console.error(`Failed to load plugin ${manifest.name}:`, e);
        }
      }
    } catch (e) {
      console.error('Failed to initialize plugin system:', e);
    }
  }

  /**
   * Load a single plugin
   */
  private async loadPlugin(manifest: PluginManifest): Promise<void> {
    this.plugins.set(manifest.name, manifest);

    // Only load frontend assets if plugin provides them
    if (!manifest.frontend) {
      return;
    }

    // Create plugin API for this plugin
    const pluginAPI = this.createPluginAPI(manifest.name);

    // Load CSS if present
    if (manifest.frontend.style) {
      await this.loadPluginCSS(manifest.name, manifest.frontend.style);
    }

    // Load JavaScript
    await this.loadPluginScript(manifest.name, manifest.frontend.script, pluginAPI);
  }

  /**
   * Create a plugin-specific API
   */
  private createPluginAPI(pluginName: string): PluginAPI {
    return {
      invoke: async (command: string, payload: any) => {
        const payloadStr = JSON.stringify(payload);
        const resultStr = await invoke<string>('plugin_invoke', {
          pluginName,
          commandName: command,
          payload: payloadStr,
        });
        return JSON.parse(resultStr);
      },

      registerModal: (id: string, component: any) => {
        const fullId = `${pluginName}:${id}`;
        this.modals.set(fullId, component);
        console.log(`Registered modal: ${fullId}`);
      },

      getCurrentRepo: async () => {
        // Import repoState dynamically to avoid circular dependencies
        try {
          const { repoState } = await import('../stores/repoState.svelte');
          return repoState.currentPath || null;
        } catch (e) {
          console.error('Failed to get current repo:', e);
          return null;
        }
      },

      getCurrentDiff: async () => {
        // Import diffSelection dynamically to avoid circular dependencies
        try {
          const { diffSelection } = await import('../stores/diffSelection.svelte');
          return diffSelection.spec || null;
        } catch (e) {
          console.error('Failed to get current diff:', e);
          return null;
        }
      },

      showModal: (id: string, props?: any) => {
        const fullId = `${pluginName}:${id}`;
        this.showModal(fullId, props);
      },

      closeModal: () => {
        this.closeModal();
      },
    };
  }

  /**
   * Load plugin CSS
   */
  private async loadPluginCSS(pluginName: string, stylePath: string): Promise<void> {
    try {
      const cssUrl = await invoke<string>('get_plugin_asset', {
        pluginName,
        assetPath: stylePath,
      });

      const link = document.createElement('link');
      link.rel = 'stylesheet';
      link.href = cssUrl;
      link.dataset.plugin = pluginName;
      document.head.appendChild(link);

      console.log(`Loaded CSS for plugin: ${pluginName}`);
    } catch (e) {
      console.error(`Failed to load CSS for plugin ${pluginName}:`, e);
    }
  }

  /**
   * Load plugin JavaScript
   */
  private async loadPluginScript(
    pluginName: string,
    scriptPath: string,
    pluginAPI: PluginAPI
  ): Promise<void> {
    return new Promise(async (resolve, reject) => {
      try {
        const scriptUrl = await invoke<string>('get_plugin_asset', {
          pluginName,
          assetPath: scriptPath,
        });

        // Initialize global plugins registry if not exists
        if (!(window as any).StagedPlugins) {
          (window as any).StagedPlugins = {};
        }

        const script = document.createElement('script');
        script.src = scriptUrl;
        script.dataset.plugin = pluginName;

        script.onload = () => {
          // Call plugin initialization function if it registered itself
          const pluginInit = (window as any).StagedPlugins[pluginName];
          if (pluginInit && typeof pluginInit === 'function') {
            try {
              pluginInit(pluginAPI);
              console.log(`Initialized plugin: ${pluginName}`);
            } catch (e) {
              console.error(`Plugin ${pluginName} initialization failed:`, e);
            }
          }
          resolve();
        };

        script.onerror = (e) => {
          reject(new Error(`Failed to load script: ${e}`));
        };

        document.head.appendChild(script);
      } catch (e) {
        reject(e);
      }
    });
  }

  /**
   * Show a modal
   */
  showModal(modalId: string, props?: any): void {
    const component = this.modals.get(modalId);
    if (!component) {
      console.error(`Modal not found: ${modalId}`);
      console.error('Available modals:', Array.from(this.modals.keys()));
      return;
    }

    // Extract plugin name from modalId (format: "pluginName:modalId")
    const pluginName = modalId.split(':')[0];

    // Create plugin API for this modal
    const pluginAPI = this.createPluginAPI(pluginName);

    const mergedProps = { ...props, api: pluginAPI };

    // Use requestAnimationFrame + setTimeout to ensure we're completely
    // outside the current execution context and any pending effects
    requestAnimationFrame(() => {
      setTimeout(() => {
        this.modalStateStore.set({
          component,
          props: mergedProps,
          visible: true,
        });
      }, 0);
    });
  }

  /**
   * Close the current modal
   */
  closeModal(): void {
    // Defer the store update to avoid effect_orphan error
    requestAnimationFrame(() => {
      setTimeout(() => {
        this.modalStateStore.set({
          component: null,
          props: {},
          visible: false,
        });
      }, 0);
    });
  }

  /**
   * Get list of loaded plugins
   */
  getPlugins(): PluginManifest[] {
    return Array.from(this.plugins.values());
  }
}

// Global singleton instance
export const pluginSystem = new PluginSystem();

// Expose globally for Plugin Settings and debugging
(window as any).pluginSystem = pluginSystem;
