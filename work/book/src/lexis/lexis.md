<!------------------------------------------------------------------------------
  This file is a part of the "Lady Deirdre" work,
  a compiler front-end foundation technology.

  This work is proprietary software with source-available code.

  To copy, use, distribute, and contribute to this work, you must agree to
  the terms of the General License Agreement:

  https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md.

  The agreement grants you a Commercial-Limited License that gives you
  the right to use my work in non-commercial and limited commercial products
  with a total gross revenue cap. To remove this commercial limit for one of
  your products, you must acquire an Unrestricted Commercial License.

  If you contribute to the source code, documentation, or related materials
  of this work, you must assign these changes to me. Contributions are
  governed by the "Derivative Work" section of the General License
  Agreement.

  Copying the work in parts is strictly forbidden, except as permitted under
  the terms of the General License Agreement.

  If you do not or cannot agree to the terms of this Agreement,
  do not use this work.

  This work is provided "as is" without any warranties, express or implied,
  except to the extent that such disclaimers are held to be legally invalid.

  Copyright (c) 2024 Ilya Lakhin (Илья Александрович Лахин).
  All rights reserved.
------------------------------------------------------------------------------->

# Lexis

Lexical scanning is the initial stage of source code text analysis.

During this process, the scanner iterates through the characters of
a Unicode string, establishing token boundaries and associating each scanned
fragment, delimited by these boundaries, with a corresponding token instance.

The lexical scanner is a simple program that implements finite-state automata,
always looking at most one character ahead. Consequently, the scanner can
be restarted at any character of the text, which is particularly beneficial for
incremental rescanning. For instance, when an end user modifies a specific
portion of the source code text, the scanner restarts before the altered
fragment, eventually converging to the state of the tail of the text.

The resulting token stream serves as input for the syntax parser.
