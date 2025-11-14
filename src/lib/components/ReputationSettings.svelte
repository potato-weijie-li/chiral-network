<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from 'svelte-i18n';
  import { invoke } from '@tauri-apps/api/core';
  import Input from '$lib/components/ui/input.svelte';
  import Label from '$lib/components/ui/label.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import { showToast } from '$lib/toast';
  import type { ReputationConfig } from '$lib/types/reputation';
  import { DEFAULT_REPUTATION_CONFIG } from '$lib/types/reputation';

  let config: ReputationConfig = { ...DEFAULT_REPUTATION_CONFIG };
  let isLoading = true;
  let isSaving = false;

  onMount(async () => {
    await loadConfig();
  });

  async function loadConfig() {
    isLoading = true;
    try {
      const loadedConfig = await invoke<ReputationConfig>('get_reputation_config');
      config = loadedConfig;
    } catch (error) {
      console.error('Failed to load reputation config:', error);
      showToast('Failed to load reputation settings', 'error');
    } finally {
      isLoading = false;
    }
  }

  async function saveConfig() {
    isSaving = true;
    try {
      await invoke('update_reputation_config', { config });
      showToast('Reputation settings saved', 'success');
    } catch (error) {
      console.error('Failed to save reputation config:', error);
      showToast('Failed to save reputation settings', 'error');
    } finally {
      isSaving = false;
    }
  }

  function resetToDefaults() {
    config = { ...DEFAULT_REPUTATION_CONFIG };
  }
</script>

<div class="space-y-4">
  {#if isLoading}
    <div class="text-center py-4">
      <div class="animate-spin h-6 w-6 border-2 border-blue-500 border-t-transparent rounded-full mx-auto"></div>
      <p class="mt-2 text-sm text-gray-500">{$t('common.loading')}</p>
    </div>
  {:else}
    <!-- Transaction Verification -->
    <div class="space-y-2">
      <h3 class="text-lg font-semibold">{$t('reputation.settings.transactionVerification')}</h3>
      
      <div class="grid grid-cols-2 gap-4">
        <div>
          <Label for="confirmation-threshold">{$t('reputation.settings.confirmationThreshold')}</Label>
          <Input
            id="confirmation-threshold"
            type="number"
            min="1"
            max="100"
            bind:value={config.confirmationThreshold}
          />
          <p class="text-xs text-gray-500 mt-1">
            {$t('reputation.settings.confirmationThresholdHelp')}
          </p>
        </div>

        <div>
          <Label for="confirmation-timeout">{$t('reputation.settings.confirmationTimeout')}</Label>
          <Input
            id="confirmation-timeout"
            type="number"
            min="60"
            max="86400"
            bind:value={config.confirmationTimeout}
          />
          <p class="text-xs text-gray-500 mt-1">
            {$t('reputation.settings.confirmationTimeoutHelp')}
          </p>
        </div>
      </div>
    </div>

    <!-- Scoring Parameters -->
    <div class="space-y-2">
      <h3 class="text-lg font-semibold">{$t('reputation.settings.scoringParameters')}</h3>
      
      <div class="grid grid-cols-2 gap-4">
        <div>
          <Label for="maturity-threshold">{$t('reputation.settings.maturityThreshold')}</Label>
          <Input
            id="maturity-threshold"
            type="number"
            min="1"
            max="1000"
            bind:value={config.maturityThreshold}
          />
          <p class="text-xs text-gray-500 mt-1">
            {$t('reputation.settings.maturityThresholdHelp')}
          </p>
        </div>

        <div>
          <Label for="decay-half-life">{$t('reputation.settings.decayHalfLife')}</Label>
          <Input
            id="decay-half-life"
            type="number"
            min="0"
            max="365"
            bind:value={config.decayHalfLife}
          />
          <p class="text-xs text-gray-500 mt-1">
            {$t('reputation.settings.decayHalfLifeHelp')}
          </p>
        </div>

        <div>
          <Label for="cache-ttl">{$t('reputation.settings.cacheTtl')}</Label>
          <Input
            id="cache-ttl"
            type="number"
            min="60"
            max="3600"
            bind:value={config.cacheTtl}
          />
          <p class="text-xs text-gray-500 mt-1">
            {$t('reputation.settings.cacheTtlHelp')}
          </p>
        </div>
      </div>
    </div>

    <!-- Blacklist Settings -->
    <div class="space-y-2">
      <h3 class="text-lg font-semibold">{$t('reputation.settings.blacklistSettings')}</h3>
      
      <div class="grid grid-cols-2 gap-4">
        <div>
          <Label for="blacklist-mode">{$t('reputation.settings.blacklistMode')}</Label>
          <select
            id="blacklist-mode"
            bind:value={config.blacklistMode}
            class="w-full px-3 py-2 border rounded-md"
          >
            <option value="manual">{$t('reputation.settings.blacklistModeManual')}</option>
            <option value="automatic">{$t('reputation.settings.blacklistModeAutomatic')}</option>
            <option value="hybrid">{$t('reputation.settings.blacklistModeHybrid')}</option>
          </select>
        </div>

        <div>
          <Label for="blacklist-auto">
            <input
              id="blacklist-auto"
              type="checkbox"
              bind:checked={config.blacklistAutoEnabled}
              class="mr-2"
            />
            {$t('reputation.settings.blacklistAutoEnabled')}
          </Label>
        </div>

        <div>
          <Label for="blacklist-score-threshold">{$t('reputation.settings.blacklistScoreThreshold')}</Label>
          <Input
            id="blacklist-score-threshold"
            type="number"
            min="0"
            max="1"
            step="0.1"
            bind:value={config.blacklistScoreThreshold}
          />
          <p class="text-xs text-gray-500 mt-1">
            {$t('reputation.settings.blacklistScoreThresholdHelp')}
          </p>
        </div>

        <div>
          <Label for="blacklist-verdicts-threshold">{$t('reputation.settings.blacklistBadVerdictsThreshold')}</Label>
          <Input
            id="blacklist-verdicts-threshold"
            type="number"
            min="1"
            max="10"
            bind:value={config.blacklistBadVerdictsThreshold}
          />
          <p class="text-xs text-gray-500 mt-1">
            {$t('reputation.settings.blacklistBadVerdictsThresholdHelp')}
          </p>
        </div>
      </div>
    </div>

    <!-- Payment Settings -->
    <div class="space-y-2">
      <h3 class="text-lg font-semibold">{$t('reputation.settings.paymentSettings')}</h3>
      
      <div class="grid grid-cols-2 gap-4">
        <div>
          <Label for="payment-deadline">{$t('reputation.settings.paymentDeadlineDefault')}</Label>
          <Input
            id="payment-deadline"
            type="number"
            min="300"
            max="86400"
            bind:value={config.paymentDeadlineDefault}
          />
          <p class="text-xs text-gray-500 mt-1">
            {$t('reputation.settings.paymentDeadlineDefaultHelp')}
          </p>
        </div>

        <div>
          <Label for="payment-grace">{$t('reputation.settings.paymentGracePeriod')}</Label>
          <Input
            id="payment-grace"
            type="number"
            min="0"
            max="3600"
            bind:value={config.paymentGracePeriod}
          />
          <p class="text-xs text-gray-500 mt-1">
            {$t('reputation.settings.paymentGracePeriodHelp')}
          </p>
        </div>

        <div>
          <Label for="min-balance">{$t('reputation.settings.minBalanceMultiplier')}</Label>
          <Input
            id="min-balance"
            type="number"
            min="1"
            max="5"
            step="0.1"
            bind:value={config.minBalanceMultiplier}
          />
          <p class="text-xs text-gray-500 mt-1">
            {$t('reputation.settings.minBalanceMultiplierHelp')}
          </p>
        </div>
      </div>
    </div>

    <!-- Action Buttons -->
    <div class="flex gap-2 justify-end pt-4 border-t">
      <Button variant="outline" on:click={resetToDefaults}>
        {$t('reputation.settings.resetToDefaults')}
      </Button>
      <Button on:click={saveConfig} disabled={isSaving}>
        {isSaving ? $t('common.saving') : $t('common.save')}
      </Button>
    </div>
  {/if}
</div>
