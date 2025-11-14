<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { t } from 'svelte-i18n';
  import Button from '$lib/components/ui/button.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import Label from '$lib/components/ui/label.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import { reputationService } from '$lib/services/reputationService';
  import { showToast } from '$lib/toast';
  import type { SignedTransactionMessage } from '$lib/types/reputation';

  export let isOpen = false;
  export let targetPeerId: string = '';

  const dispatch = createEventDispatcher();

  let complaintType: 'non-payment' | 'non-delivery' | 'other' = 'non-payment';
  let details = '';
  let submitOnChain = false;
  let isSubmitting = false;

  // Evidence fields
  let signedMessageJson = '';
  let deliveryProofJson = '';
  let transferLogJson = '';
  let protocolLogJson = '';

  function close() {
    isOpen = false;
    dispatch('close');
  }

  async function handleSubmit() {
    if (!targetPeerId.trim()) {
      showToast('Please enter a peer ID', 'error');
      return;
    }

    isSubmitting = true;
    try {
      // Parse evidence
      const evidence: any = {};

      if (signedMessageJson.trim()) {
        try {
          evidence.signedTransactionMessage = JSON.parse(signedMessageJson) as SignedTransactionMessage;
        } catch (e) {
          showToast('Invalid signed transaction message JSON', 'error');
          isSubmitting = false;
          return;
        }
      }

      if (deliveryProofJson.trim()) {
        try {
          evidence.deliveryProof = JSON.parse(deliveryProofJson);
        } catch (e) {
          showToast('Invalid delivery proof JSON', 'error');
          isSubmitting = false;
          return;
        }
      }

      if (transferLogJson.trim()) {
        try {
          evidence.transferCompletionLog = JSON.parse(transferLogJson);
        } catch (e) {
          showToast('Invalid transfer log JSON', 'error');
          isSubmitting = false;
          return;
        }
      }

      if (protocolLogJson.trim()) {
        try {
          evidence.protocolLogs = JSON.parse(protocolLogJson);
        } catch (e) {
          showToast('Invalid protocol log JSON', 'error');
          isSubmitting = false;
          return;
        }
      }

      await reputationService.fileComplaint(
        targetPeerId.trim(),
        complaintType,
        evidence,
        submitOnChain
      );

      showToast('Complaint filed successfully', 'success');
      dispatch('submitted', { targetPeerId, complaintType });
      close();
    } catch (error) {
      console.error('Failed to file complaint:', error);
      showToast(`Failed to file complaint: ${error}`, 'error');
    } finally {
      isSubmitting = false;
    }
  }
</script>

<Modal bind:isOpen on:close={close}>
  <div class="p-6 space-y-4">
    <h2 class="text-2xl font-bold">{$t('reputation.complaint.title')}</h2>
    
    <div class="space-y-4">
      <!-- Target Peer -->
      <div>
        <Label for="target-peer">{$t('reputation.complaint.targetPeer')}</Label>
        <Input
          id="target-peer"
          bind:value={targetPeerId}
          placeholder={$t('reputation.complaint.targetPeerPlaceholder')}
          disabled={isSubmitting}
        />
      </div>

      <!-- Complaint Type -->
      <div>
        <Label for="complaint-type">{$t('reputation.complaint.type')}</Label>
        <select
          id="complaint-type"
          bind:value={complaintType}
          class="w-full px-3 py-2 border rounded-md"
          disabled={isSubmitting}
        >
          <option value="non-payment">{$t('reputation.complaint.typeNonPayment')}</option>
          <option value="non-delivery">{$t('reputation.complaint.typeNonDelivery')}</option>
          <option value="other">{$t('reputation.complaint.typeOther')}</option>
        </select>
      </div>

      <!-- Details -->
      <div>
        <Label for="details">{$t('reputation.complaint.details')}</Label>
        <textarea
          id="details"
          bind:value={details}
          placeholder={$t('reputation.complaint.detailsPlaceholder')}
          rows="3"
          class="w-full px-3 py-2 border rounded-md"
          disabled={isSubmitting}
        ></textarea>
      </div>

      <!-- Evidence Section -->
      <div class="border-t pt-4">
        <h3 class="font-semibold mb-2">{$t('reputation.complaint.evidence')}</h3>
        <p class="text-sm text-gray-500 mb-3">{$t('reputation.complaint.evidenceHelp')}</p>

        {#if complaintType === 'non-payment'}
          <div class="space-y-3">
            <div>
              <Label for="signed-message">{$t('reputation.complaint.signedMessage')}</Label>
              <textarea
                id="signed-message"
                bind:value={signedMessageJson}
                placeholder='{"from": "0x...", "to": "0x...", ...}'
                rows="4"
                class="w-full px-3 py-2 border rounded-md font-mono text-xs"
                disabled={isSubmitting}
              ></textarea>
              <p class="text-xs text-gray-500 mt-1">{$t('reputation.complaint.signedMessageHelp')}</p>
            </div>

            <div>
              <Label for="delivery-proof">{$t('reputation.complaint.deliveryProof')}</Label>
              <textarea
                id="delivery-proof"
                bind:value={deliveryProofJson}
                placeholder='{"chunks": [...], "merkleRoot": "..."}'
                rows="3"
                class="w-full px-3 py-2 border rounded-md font-mono text-xs"
                disabled={isSubmitting}
              ></textarea>
            </div>
          </div>
        {/if}

        {#if complaintType === 'non-delivery'}
          <div class="space-y-3">
            <div>
              <Label for="transfer-log">{$t('reputation.complaint.transferLog')}</Label>
              <textarea
                id="transfer-log"
                bind:value={transferLogJson}
                placeholder='{"startTime": ..., "endTime": ..., ...}'
                rows="3"
                class="w-full px-3 py-2 border rounded-md font-mono text-xs"
                disabled={isSubmitting}
              ></textarea>
            </div>
          </div>
        {/if}

        <div class="mt-3">
          <Label for="protocol-logs">{$t('reputation.complaint.protocolLogs')}</Label>
          <textarea
            id="protocol-logs"
            bind:value={protocolLogJson}
            placeholder='{"messages": [...], "connections": [...]}'
            rows="3"
            class="w-full px-3 py-2 border rounded-md font-mono text-xs"
            disabled={isSubmitting}
          ></textarea>
        </div>
      </div>

      <!-- Submit Options -->
      <div class="border-t pt-4">
        <Label>
          <input
            type="checkbox"
            bind:checked={submitOnChain}
            disabled={isSubmitting}
            class="mr-2"
          />
          {$t('reputation.complaint.submitOnChain')}
        </Label>
        <p class="text-xs text-gray-500 mt-1">{$t('reputation.complaint.submitOnChainHelp')}</p>
      </div>
    </div>

    <!-- Action Buttons -->
    <div class="flex gap-2 justify-end pt-4 border-t">
      <Button variant="outline" on:click={close} disabled={isSubmitting}>
        {$t('common.cancel')}
      </Button>
      <Button on:click={handleSubmit} disabled={isSubmitting}>
        {isSubmitting ? $t('reputation.complaint.submitting') : $t('reputation.complaint.submit')}
      </Button>
    </div>
  </div>
</Modal>
