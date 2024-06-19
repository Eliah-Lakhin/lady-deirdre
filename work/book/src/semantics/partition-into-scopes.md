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

# Partition into Scopes

To achieve efficiency in incremental semantic analysis, the language should
allow for partitioning of the source code of compilation units into scopes.
Within these scopes, local base semantic facts can be inferred relatively
independently from other scopes.

For instance, in the Java programming language, each compilation unit (file)
introduces a Java class, which can be segmented into several abstract semantic
layers:

- The class type declaration layer.
- The declaration layer for class members (fields and methods) within the class.
- The layer for implementing methods (method bodies).

Initially, we can consider each of these layers as independent from each other:
each method's body code constitutes an independent scope, each class member
signature forms an independent scope, and finally, the class type declaration
stands as a scope initially separate from its members and method
implementations.

<pre>
<code>
class <span style="background: color-mix(in srgb, cyan, white 80%);">MyClass&ltT&gt</span> {
    <span style="background: color-mix(in srgb, lightsalmon, white 80%);">private T fieldFoo</span> = <span style="background: color-mix(in srgb, lightgreen, white 60%);">5</span>;
    
    <span style="background: color-mix(in srgb, lightsalmon, white 80%);">public void methodBar(int x)</span> <span style="background: color-mix(in srgb, lightgreen, white 60%);">{
        //..
    }</span>
    
    <span style="background: color-mix(in srgb, lightsalmon, white 80%);">public void methodBaz(T y)</span> <span style="background: color-mix(in srgb, lightgreen, white 60%);">{
        //..
    }</span>
}
</code>
</pre>

From each of these scopes, we infer as much localized information as needed,
which we can later utilize to draw more comprehensive conclusions.

In the example above, from the signature of `methodBaz`, we can deduce that it
possesses a parameter of type `T`. However, solely from this declaration, we
cannot pinpoint where exactly this type has been declared. Conversely, from the
signature of `MyClass`, we gather that the class has a generic type `T` that can
be utilized within its members. Yet, we cannot determine solely from the type
signature declaration which class members specifically employ this type. By
linking these two independent pieces of information together, we can conclude
that the parameter `x` of `methodBaz` has a generic type declared in the class
signature.

In terms of Lady Deirdre's semantic analysis framework, the localized facts we
infer from the scopes constitute the inputs of the language's *semantic model*.

The semantic graph attributes, which map from the scope nodes to the semantic
model objects, serve as the entry points of the model (the input attributes).
Other attributes deduce more generalized facts based on the state of the model.

The granularity of the attributes within the semantic graph and the separation
of scopes in the programming language syntax are core features that render
incremental semantic analysis efficient.
