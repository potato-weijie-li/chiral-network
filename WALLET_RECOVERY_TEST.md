# Wallet Recovery from 12-Word Mnemonic - Testing Guide

## Feature Summary
Added ability to recover an existing wallet from a 12-word or 24-word mnemonic phrase during first-run setup.

## Changes Made
1. **FirstRunWizard.svelte** - Added 'recover' mode and recovery button
2. **en.json** - Added i18n strings for recovery UI

## How to Test

### Manual Testing Steps:

1. **Reset to first-run state:**
   - Delete wallet data to trigger FirstRunWizard
   - Or test in Account page with "Import from Recovery Phrase" button

2. **Test Recovery Flow:**
   - Launch app → Welcome screen appears
   - Click "Recover from Recovery Phrase" button (new!)
   - Enter a valid 12 or 24-word mnemonic
   - (Optional) Enter passphrase if used
   - Click "Import"
   - Wallet should be recovered with correct address

3. **Test Validation:**
   - Invalid mnemonic → Should show error
   - Wrong number of words (e.g., 11 or 13) → Should show error
   - Wrong checksum → Should show error

### Test Mnemonic (for testing only - DO NOT USE for real funds)
```
abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about
```
Expected address: `0x7608d2d3219132ffe9c78c3ac3496f9018e77b53`

**Note**: Address is derived using BIP44 path `m/44'/<chainId>'/0'/0/0` where chainId is your Chiral Network chain ID (default: 98765). This differs from standard Ethereum which uses coinType 60.

### Existing Functionality (should still work):
- ✅ Create New Wallet → generates new 12/24-word phrase
- ✅ Import Existing Wallet → imports via private key
- ✅ Create Test Wallet (dev mode) → quick test wallet

## Code Statistics
- **Lines Added**: ~30 lines
- **Files Modified**: 2 files
  - FirstRunWizard.svelte (+15 lines)
  - en.json (+13 lines)
- **Files Created**: 0
- **Reused Code**: MnemonicWizard import mode (no changes needed)

## Architecture
```
Welcome Screen
    ├─ Create New Wallet → MnemonicWizard (mode='create')
    ├─ Import Existing Wallet → Private Key Input
    └─ Recover from Recovery Phrase → MnemonicWizard (mode='import') ← NEW!
```

## Technical Details

### Mode Flow:
- `mode='welcome'` → Initial screen
- `mode='mnemonic'` → Create new wallet (generates mnemonic)
- `mode='import'` → Import via private key
- `mode='recover'` → **NEW** - Import via mnemonic recovery

### Mnemonic Mode:
- `mnemonicMode='create'` → Generate new phrase
- `mnemonicMode='import'` → Import existing phrase (recovery)

## Related Files
- `/src/lib/wallet/bip39.ts` - BIP39 mnemonic implementation
- `/src/lib/wallet/hd.ts` - HD wallet derivation (BIP32/BIP44)
- `/src/lib/components/wallet/MnemonicWizard.svelte` - Mnemonic UI (reused)

## Known Limitations
- Only supports English wordlist (EN)
- Hardcoded BIP44 derivation path: m/44'/60'/0'/0/0
- No support for custom derivation paths
- No support for multiple accounts during recovery (derives only first account)

## Future Enhancements
- Multi-language wordlist support
- Custom derivation path selection
- Batch account derivation during recovery
- Mnemonic strength selection (12/15/18/21/24 words)
