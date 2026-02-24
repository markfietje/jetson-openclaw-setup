// Add these lines after line 557 in receiver_loop:

// DEBUG: Log what we received
tracing::info!("[RECEIVER] Content body type: {:?}", match &c.body {
    ContentBody::DataMessage(_) => "DataMessage",
    ContentBody::SynchronizeMessage(_) => "SynchronizeMessage",  
    ContentBody::TypingMessage(_) => "TypingMessage",
    ContentBody::CallMessage(_) => "CallMessage",
    _ => "Other",
});

// DEBUG: Log the result of process_content
let result = Self::process_content(&c, account_number.lock().clone());
tracing::info!("[RECEIVER] process_content returned: {:?}", result.is_some());
