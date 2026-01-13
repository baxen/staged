/**
 * Plugin system for Stage frontend
 *
 * Handles dynamic loading of plugin JavaScript/CSS assets and provides
 * an API for plugins to interact with the Stage application.
 */

import { invoke } from '@tauri-apps/api/core';
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
  private modalState: ModalState = {
    component: null,
    props: {},
    visible: false,
  };
  private modalStateCallbacks: Array<(state: ModalState) => void> = [];

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
        // TODO: Get from global state
        return null;
      },

      getCurrentDiff: async () => {
        // TODO: Get from global state
        return null;
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
      return;
    }

    this.modalState = {
      component,
      props: props || {},
      visible: true,
    };

    this.notifyModalStateChange();
  }

  /**
   * Close the current modal
   */
  closeModal(): void {
    this.modalState = {
      component: null,
      props: {},
      visible: false,
    };

    this.notifyModalStateChange();
  }

  /**
   * Get current modal state
   */
  getModalState(): ModalState {
    return { ...this.modalState };
  }

  /**
   * Subscribe to modal state changes
   */
  onModalStateChange(callback: (state: ModalState) => void): () => void {
    this.modalStateCallbacks.push(callback);

    // Return unsubscribe function
    return () => {
      const index = this.modalStateCallbacks.indexOf(callback);
      if (index > -1) {
        this.modalStateCallbacks.splice(index, 1);
      }
    };
  }

  /**
   * Notify all subscribers of modal state change
   */
  private notifyModalStateChange(): void {
    const state = this.getModalState();
    for (const callback of this.modalStateCallbacks) {
      try {
        callback(state);
      } catch (e) {
        console.error('Modal state callback error:', e);
      }
    }
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
