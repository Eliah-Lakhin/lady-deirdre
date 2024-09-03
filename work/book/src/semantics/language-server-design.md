<!------------------------------------------------------------------------------
  This file is part of "Lady Deirdre", a compiler front-end foundation
  technology.

  This work is proprietary software with source-available code.

  To copy, use, distribute, or contribute to this work, you must agree to
  the terms of the General License Agreement:

  https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md

  The agreement grants a Basic Commercial License, allowing you to use
  this work in non-commercial and limited commercial products with a total
  gross revenue cap. To remove this commercial limit for one of your
  products, you must acquire a Full Commercial License.

  If you contribute to the source code, documentation, or related materials,
  you must grant me an exclusive license to these contributions.
  Contributions are governed by the "Contributions" section of the General
  License Agreement.

  Copying the work in parts is strictly forbidden, except as permitted
  under the General License Agreement.

  If you do not or cannot agree to the terms of this Agreement,
  do not use this work.

  This work is provided "as is", without any warranties, express or implied,
  except where such disclaimers are legally invalid.

  Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).
  All rights reserved.
------------------------------------------------------------------------------->

## Language Server Design

The rationale behind the Analyzer's complex data access API is that it is
specifically designed for use in language server programs that handle language
client (code editor) requests concurrently.

The client of
the [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)
notifies the server about
various events happening on the client side, allowing the server to handle these
requests concurrently in dedicated working threads.

For example, when the client notifies the server that the end user has opened a
file in the editor by sending the source code text, you can acquire a mutation
task and create a Document to represent this task.

When the end-user edits the file, the client usually sends a notification to the
server containing an edited fragment span and the text the user is typing. In
this case, you would acquire a mutation task and apply the edit to the
corresponding document.

When the user scrolls the document window, clicks or moves the cursor over
symbols in the source code, or requests code completion suggestions, the client
sends multiple requests to the server asking for various semantic facts about
the source code spans that the user is currently observing. The server can use
analysis tasks to query the Analyzer's document semantics and respond to these
requests accordingly.

The observation requests from the client can be canceled if the client decides
that a request is no longer relevant. In this case, once the server receives the
cancellation notification, it can signal the corresponding working thread to
interrupt its job by triggering the task handle used by the working thread.

Client-side requests can obviously conflict with each other. For example, an
incoming document edit notification would conflict with in-progress semantic
analysis requests.

These conflicts can be resolved through the Analyzer's task priority system.
Analysis tasks used to handle client analysis requests should typically have
lower priorities than mutation tasks handling document edits, as immediate
synchronization of changes in the source code file on the client side with the
server-side state is more important than analysis jobs.

Since the analysis job is subject to frequent interruptions by client-side
cancellation notifications and mutation jobs, the typical analysis job workflow
involves a loop with the following steps:

1. At the beginning of the loop, check if the client-side request has been
   canceled. If it has, break the loop and respond to the client accordingly.
2. Otherwise, acquire an analysis task from the Analyzer and execute the actual
   analysis procedure based on the client request inputs.
3. If the analysis succeeds, return the response to the client with the analysis
   results and finish the loop.
4. If the analysis job is interrupted because another thread with a higher
   priority attempts to acquire a conflicting (mutation) task object, the
   analysis worker should drop its analysis task object to allow the other
   thread to fulfill its request[^delay]. Then, restart the loop from step one
   to eventually complete the client-side analysis request.

An important feature of the above procedure is that even if we drop the analysis
task in the middle of its execution, the Analyzer may still manage to complete
part of the semantic graph validations. When the analysis procedure resumes, it
is likely to execute much faster, continuing validation from the point where it
was interrupted. This approach is particularly relevant for computation-heavy
analysis procedures on highly granular semantic models.

[^delay]: At this step, you can even park or sleep the current thread for a
short amount of time to ensure that the other thread acquires the requested task
without race conditions.
