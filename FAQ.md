# Licensing Terms F.A.Q.

> **Disclaimer**: The provided material is for general informational purposes
  only and is not intended to be legal advice. Please consult with your own
  legal counsel regarding your situation and specific legal questions you may
  have. In case of a conflict or inconsistency between this F.A.Q. and the
  [General License Agreement](https://github.com/Eliah-Lakhin/lady-deirdre/blob/master/EULA.md),
  the agreement prevails.

## Can you give a brief overview of your licensing terms?

Users of my software include businesses and programmers like me.

By publishing Lady Deirdre in the form of source code, I aim to be transparent with
my users, provide my work for public audit, and share knowledge with the
software development community.

Startups can begin using Lady Deirdre free of charge to develop a commercial
product, and if it succeeds, after the product generates a certain amount of
revenue, they should purchase a separate license from me at a reasonable price
to continue using the product commercially.

To support the creative endeavors of programmers in developing non-commercial
software, my licensing terms grant the necessary rights to allow the development
of creative projects based on Lady Deirdre and distribute them in source code form.

## What are the license options?

The General License Agreement defines two types of licenses:

1. **Basic Commercial License**, which is granted by default, free of charge,
   but has certain restrictions on commercial use of the work.
2. **Full Commercial License**, which is available for separate purchase and
   removes the commercial restrictions of the Basic license.

Both licenses allow you to develop commercial or non-commercial software based
on Lady Deirdre. The main difference between them is the total gross revenue you can
earn with your product during its lifecycle.

| License   | Max No. of Products   | Duration  | Max Gross Revenue  | Acquiring                                                                                                                                                                                                       |
|-----------|-----------------------|-----------|--------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **Basic** | Unlimited             | Perpetual | Up to $200,000 USD | Granted Automatically                                                                                                                                                                                           |
| **Full**  | 1 Product per License | Annual    | Unlimited          | [Available for Purchase](https://www.patreon.com/lakhin/shop/lady-deirdre-annual-full-commercial-240505?utm_medium=clipboard_copy&utm_source=copyLink&utm_campaign=productshare_fan&utm_content=join_link) |

## What happens when the Full Commercial License expires?

You should renew the license to continue using new versions and upgrades of
my work.

If you do not renew the license, you can keep using the version you have at the
time of expiration in your software product perpetually.

## I want to create a project based on Lady Deirdre and publish it in source code. How can I do that?

You can develop a full-featured software or an API extension that interacts
with the API of my crates from [crates.io](https://crates.io/crates/lady-deirdre),
just as you would in any Rust project.

You can then publish it in source code form and/or on crates.io as well, and
distribute it under your own terms, for example, under a permissive license
such as MIT, provided that your license covers only your work and does not
cover my work.

In this scenario, the end user will acquire a separate license for your work
from you and a separate license for my work from me.

To ensure transparency with your users and to avoid possible misunderstandings
you can specify that your license covers only your work, and that a license for
Lady Deirdre needs to be acquired separately.

However, my work is provided free of charge for personal use. Therefore, your
users will be able to acquire a license from me for personal use of my work
without additional fees.

## What if my crate is used in commercial software?

In this case, the authors of the commercial software should acquire a license
from you to use your work and a license from me to use my work, which is subject
to commercial options.

## How can I distribute compiled executables?

You can compile the source code of my work together with your source code or any
other code you have legal access to into a single executable program and
distribute this program on your own terms as part of your commercial or
non-commercial product.

When distributing in the form of compiled executables, your users do not need
to obtain a license from me to use Lady Deirdre, and you can distribute your product
fully on your own terms.

Additionally, you are not required to distribute this product together with the
source code. Your product can be a closed-source program.

However, if you distribute a commercial product, this product is subject to the
commercial limitations of the *Basic Commercial License*, and you are
recommended to acquire the *Full Commercial License* in advance to remove these
restrictions. You can purchase this license on my [Patreon Page](https://www.patreon.com/lakhin/shop/lady-deirdre-annual-full-commercial-240505?utm_medium=clipboard_copy&utm_source=copyLink&utm_campaign=productshare_fan&utm_content=join_link).

## What kind of Lady Deirdre license should I choose for my non-commercial project?

In most cases, the *Basic Commercial License*, which is granted automatically
and is free of charge, will cover your needs.

However, if you earn donations or receive funds through crowdfunding campaigns
for your project, you should be aware that these sources of funding are
considered revenue as well.

If the total amount of earnings does not exceed $200,000 USD, the Basic license
remains in full effect. But if your earnings exceed this amount, you are
required to acquire a *Full Commercial License* from me, which you can purchase
on my [Patreon Page](https://www.patreon.com/lakhin/shop/lady-deirdre-annual-full-commercial-240505?utm_medium=clipboard_copy&utm_source=copyLink&utm_campaign=productshare_fan&utm_content=join_link).

## Who owns the project that I develop using Lady Deirdre?

As long as you don't modify my work's source code and use it solely through the
Lady Deirdre crates' public API (e.g., by linking to
[crates.io](https://crates.io/crates/lady-deirdre)), you own the project you develop.

This includes both the source code you develop and the compiled executables.

## May I contribute to your project?

If you find a bug or have a feature suggestion, you can open a pull request
in my GitHub repository.

Please note that my work is proprietary software, intended for solo development.
For this reason, the agreement requires you to grant me an exclusive license
to any changes you make to my project's source code.

However, if you want to create an extension for my crate, you can develop a
separate crate that uses my crate's public APIs through Cargo. In this case, you
do not need to grant me an exclusive license to your work, and you can
distribute your project under any permissive license, such as the MIT license.

I have deliberately designed my crate's APIs to be extendable for third-party
authors who want to create dedicated Lady Deirdre extensions.
