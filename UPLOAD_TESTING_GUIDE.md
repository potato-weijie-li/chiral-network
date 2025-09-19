# Chiral Network Upload Feature Testing Guide

## Overview

The upload feature has been implemented with a BitTorrent-like instant seeding model. Files are immediately available to the network upon upload, following the architecture described in the documentation.

## Testing Setup

### Prerequisites

1. **System Requirements**:
   - Node.js 18+ installed
   - Rust toolchain installed
   - Git

2. **Build the Application**:
   ```bash
   git clone https://github.com/potato-weijie-li/chiral-network.git
   cd chiral-network
   npm install
   npm run tauri dev
   ```

   The application will start in development mode with the Tauri desktop app.

## Testing the Upload Feature

### 1. Basic Upload Flow

1. **Navigate to Upload Page**:
   - Open the application
   - Click on "Upload" in the navigation (should be the default page)

2. **Upload a File**:
   - **Method 1**: Drag and drop a file onto the drop zone
   - **Method 2**: Click "Add Files" button and select files

3. **Observe Status Progression**:
   - File appears with blue "uploading" badge
   - Changes to yellow "verifying" badge
   - Finally shows green "seeding" badge when complete

4. **Verify File Information**:
   - File name, size, and hash are displayed
   - Seeder count shows 1 (your node)
   - Hash can be copied by clicking the copy icon

### 2. Advanced Testing Scenarios

#### Multiple File Upload
- Select or drag multiple files at once
- Each file should process independently
- Status updates should occur for each file individually

#### Duplicate Detection
- Try uploading the same file twice
- Should show warning toast about duplicate being skipped
- Duplicate file should not appear in the list

#### Error Handling
- Try uploading a very large file (>100MB)
- Network errors should show appropriate error messages
- Failed uploads display red "failed" badge

#### File Management
- Click the X button to remove files from sharing
- Files should disappear from the upload list
- No longer available for seeding

### 3. Backend Integration Testing

#### File Transfer Service
1. **Check Service Status**:
   - Upload should automatically start file transfer service
   - Console logs should show service initialization

2. **Verify Storage**:
   - Files are stored in memory for immediate availability
   - File hashes are calculated using SHA-256

#### DHT Integration
1. **Start DHT Service**:
   - Navigate to Network page
   - Click "Start DHT Node" if not already running

2. **Verify Registration**:
   - Upload a file after DHT is running
   - Check console logs for DHT registration messages
   - File metadata should be published to the DHT

### 4. UI/UX Testing

#### Visual Feedback
- Drag and drop highlighting works correctly
- Status badges are color-coded appropriately:
  - Blue: Uploading
  - Yellow: Verifying storage
  - Green: Seeding (ready)
  - Red: Failed

#### Responsiveness
- Interface works on different window sizes
- File cards layout adjusts appropriately
- Buttons and interactions remain accessible

## Expected Behavior

### Instant Seeding Model
Unlike traditional file sharing where you "upload" to a server, Chiral Network implements instant seeding:

1. **Immediate Availability**: Files start seeding as soon as upload completes
2. **No Upload Queue**: Files don't wait in a queue, they're immediately processed
3. **Continuous Seeding**: Files remain available until manually removed
4. **Local Storage**: Files are stored locally and made available to the network

### Network Integration
- Files are registered in the DHT for discovery
- Storage nodes can be queried for file availability
- Verification ensures files are properly stored and accessible

## Troubleshooting

### Common Issues

1. **Upload Button Not Working**:
   - Check browser console for errors
   - Ensure Tauri backend is running
   - Try restarting the application

2. **Files Stuck in "Uploading"**:
   - Check if file transfer service started
   - Look for error messages in console
   - Try with smaller files first

3. **DHT Not Working**:
   - Ensure DHT service is started from Network page
   - Check firewall settings for P2P ports
   - Verify bootstrap nodes are accessible

4. **Storage Verification Fails**:
   - Files may show "verifying" status indefinitely
   - Check backend logs for storage errors
   - Restart services if needed

### Debug Information

#### Console Logs
Look for these log patterns:
```
[INFO] File uploaded and instantly available for seeding: <hash>
[INFO] File metadata published to DHT successfully
[INFO] File <hash> verified in both local storage and DHT
```

#### Service Status
- File Transfer Service: Auto-starts when needed
- DHT Service: Must be manually started from Network page
- Storage verification: Happens automatically after upload

## Technical Implementation Details

### File Flow
```
User selects file
    ↓
Browser calculates quick hash (duplicate check)
    ↓
Tauri backend processes file
    ↓
File stored in memory (instant seeding starts)
    ↓
DHT registration (if DHT service running)
    ↓
Storage verification
    ↓
UI shows "seeding" status
```

### Backend Commands Used
- `upload_file_data_to_network`: Main upload command
- `verify_file_storage`: Confirms file is properly stored
- `get_file_upload_status`: Gets upload progress/status
- `start_file_transfer_service`: Initializes file services

## Security Considerations

- Files are hashed using SHA-256 for integrity
- Duplicate detection prevents accidental re-uploads
- No sensitive data is logged
- Files remain local until explicitly shared

## Performance Notes

- Upload performance depends on file size and system resources
- Large files (>100MB) may take longer to process
- Multiple simultaneous uploads are supported
- Memory usage increases with number of seeded files

## Next Steps

After basic testing, consider:
1. Testing with different file types and sizes
2. Verifying network connectivity between multiple nodes
3. Testing download functionality from other nodes
4. Monitoring resource usage during heavy upload activity

This implementation provides a solid foundation for the BitTorrent-like file sharing system described in the project documentation.