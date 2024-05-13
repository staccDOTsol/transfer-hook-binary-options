import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { TransferHook } from "../target/types/transfer_hook";
import {
  PublicKey,
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction,
  Keypair,
} from "@solana/web3.js";
import {
  ExtensionType,
  TOKEN_2022_PROGRAM_ID,
  getMintLen,
  createInitializeMintInstruction,
  createInitializeTransferHookInstruction,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createAssociatedTokenAccountInstruction,
  createMintToInstruction,
  getAssociatedTokenAddressSync,
  createApproveInstruction,
  createSyncNativeInstruction,
  NATIVE_MINT,
  TOKEN_PROGRAM_ID,
  getAccount,
  getOrCreateAssociatedTokenAccount,
  createTransferCheckedWithTransferHookInstruction,
  getMint,
  getTransferHook,
  getExtraAccountMetaAddress,
  getExtraAccountMetas,
} from "@solana/spl-token";
import assert from "assert";

describe("transfer-hook", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = new Program(
    {
      address: "AZR4kEoxHrD879oPU5vLbJnryCHEyrJfiFwmASUXdFqf",
      metadata: {
        name: "transfer_hook",
        version: "0.1.0",
        spec: "0.1.0",
        description: "Created with Anchor",
      },
      instructions: [
        {
          name: "burn_baby_burn",
          discriminator: [122, 217, 136, 181, 133, 220, 18, 161],
          accounts: [
            {
              name: "payer",
              writable: true,
              signer: true,
            },
            {
              name: "mint",
              writable: true,
            },
            {
              name: "mint_ata",
              writable: true,
            },
            {
              name: "game",
              writable: true,
              pda: {
                seeds: [
                  {
                    kind: "const",
                    value: [103, 97, 109, 101],
                  },
                  {
                    kind: "account",
                    path: "mint",
                  },
                ],
              },
            },
            {
              name: "token_program",
              address: "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
            },
          ],
          args: [
            {
              name: "amount",
              type: "u64",
            },
          ],
        },
        {
          name: "initialize_extra_account_meta_list",
          discriminator: [92, 197, 174, 197, 41, 124, 19, 3],
          accounts: [
            {
              name: "payer",
              writable: true,
              signer: true,
            },
            {
              name: "extra_account_meta_list",
              writable: true,
              pda: {
                seeds: [
                  {
                    kind: "const",
                    value: [
                      101, 120, 116, 114, 97, 45, 97, 99, 99, 111, 117, 110,
                      116, 45, 109, 101, 116, 97, 115,
                    ],
                  },
                  {
                    kind: "account",
                    path: "mint",
                  },
                ],
              },
            },
            {
              name: "mint",
              writable: true,
            },
            {
              name: "other_mint",
            },
            {
              name: "game",
              writable: true,
              pda: {
                seeds: [
                  {
                    kind: "const",
                    value: [103, 97, 109, 101],
                  },
                  {
                    kind: "account",
                    path: "mint",
                  },
                ],
              },
            },
            {
              name: "system_program",
              address: "11111111111111111111111111111111",
            },
            {
              name: "raydium_clmm",
            },
            {
              name: "token_program",
              address: "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
            },
          ],
          args: [],
        },
        {
          name: "om_nom_nom",
          discriminator: [243, 58, 68, 27, 183, 79, 41, 125],
          accounts: [
            {
              name: "payer",
              writable: true,
              signer: true,
            },
            {
              name: "game",
              writable: true,
              pda: {
                seeds: [
                  {
                    kind: "const",
                    value: [103, 97, 109, 101],
                  },
                  {
                    kind: "account",
                    path: "mint",
                  },
                ],
              },
            },
            {
              name: "other_mint",
              writable: true,
            },
            {
              name: "mint",
              writable: true,
            },
            {
              name: "token_program",
              address: "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
            },
          ],
          args: [],
        },
        {
          name: "transfer_hook",
          discriminator: [220, 57, 220, 152, 126, 125, 97, 168],
          accounts: [
            {
              name: "source_token",
            },
            {
              name: "mint",
            },
            {
              name: "destination_token",
            },
            {
              name: "owner",
            },
            {
              name: "extra_account_meta_list",
              pda: {
                seeds: [
                  {
                    kind: "const",
                    value: [
                      101, 120, 116, 114, 97, 45, 97, 99, 99, 111, 117, 110,
                      116, 45, 109, 101, 116, 97, 115,
                    ],
                  },
                  {
                    kind: "account",
                    path: "mint",
                  },
                ],
              },
            },
            {
              name: "other_mint",
            },
            {
              name: "game",
              writable: true,
              pda: {
                seeds: [
                  {
                    kind: "const",
                    value: [103, 97, 109, 101],
                  },
                  {
                    kind: "account",
                    path: "mint",
                  },
                ],
              },
            },
            {
              name: "raydium_clmm",
            },
          ],
          args: [
            {
              name: "_amount",
              type: "u64",
            },
          ],
        },
      ],
      accounts: [
        {
          name: "Game",
          discriminator: [27, 90, 166, 125, 74, 100, 121, 18],
        },
      ],
      errors: [
        {
          code: 6000,
          name: "InvalidCLMMOracle",
          msg: "Invalid CLMM Oracle",
        },
      ],
      types: [
        {
          name: "Game",
          type: {
            kind: "struct",
            fields: [
              {
                name: "this_mint_won",
                type: "bool",
              },
              {
                name: "this_mint_ate_the_other_already",
                type: "bool",
              },
              {
                name: "total_pending_payout",
                type: "u64",
              },
              {
                name: "next_epoch",
                type: "u64",
              },
              {
                name: "last_epoch",
                type: "u64",
              },
              {
                name: "last_price",
                type: "u64",
              },
              {
                name: "other_mint",
                type: "pubkey",
              },
              {
                name: "raydium_clmm",
                type: "pubkey",
              },
            ],
          },
        },
      ],
    } as anchor.Idl,
    provider
  );
  const wallet = provider.wallet as anchor.Wallet;
  const connection = provider.connection;

  // Generate keypair to use as address for the transfer-hook enabled mint
  const mint = new Keypair();
  const mint2 = new Keypair();

  const decimals = 9;

  // Sender token account address
  const sourceTokenAccount = getAssociatedTokenAddressSync(
    mint.publicKey,
    wallet.publicKey,
    false,
    TOKEN_2022_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID
  );

  // Sender token account address
  const sourceTokenAccount2 = getAssociatedTokenAddressSync(
    mint2.publicKey,
    wallet.publicKey,
    false,
    TOKEN_2022_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID
  );

  // ExtraAccountMetaList address
  // Store extra accounts required by the custom transfer hook instruction
  const [extraAccountMetaListPDA1] = PublicKey.findProgramAddressSync(
    [Buffer.from("extra-account-metas"), mint.publicKey.toBuffer()],
    program.programId
  );

  // ExtraAccountMetaList address
  // Store extra accounts required by the custom transfer hook instruction
  const [extraAccountMetaListPDA2] = PublicKey.findProgramAddressSync(
    [Buffer.from("extra-account-metas"), mint2.publicKey.toBuffer()],
    program.programId
  );

  // PDA delegate to transfer wSOL tokens from sender
  const [delegatePDA1] = PublicKey.findProgramAddressSync(
    [Buffer.from("game"), mint.publicKey.toBuffer()],
    program.programId
  );
  let upOption = mint.publicKey > mint2.publicKey;
  if (upOption) {
    console.log("mint is greater than mint2", mint.publicKey.toBase58());
  } else {
    console.log("mint2 is greater than mint", mint2.publicKey.toBase58());
  }
  const [delegatePDA2] = PublicKey.findProgramAddressSync(
    [Buffer.from("game"), mint2.publicKey.toBuffer()],
    program.programId
  );

  // Sender wSOL token account address
  const senderWSolTokenAccount = getAssociatedTokenAddressSync(
    NATIVE_MINT, // mint
    wallet.publicKey // owner
  );

  // Create the two WSol token accounts as part of setup
  before(async () => {
    // WSol Token Account for sender
    await getOrCreateAssociatedTokenAccount(
      connection,
      wallet.payer,
      NATIVE_MINT,
      wallet.publicKey
    );
  });

  it("Create Mint Account with Transfer Hook Extension", async () => {
    const extensions = [ExtensionType.TransferHook];
    const mintLen = getMintLen(extensions);
    const lamports =
      await provider.connection.getMinimumBalanceForRentExemption(mintLen);

    let transaction = new Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: wallet.publicKey,
        newAccountPubkey: mint2.publicKey,
        space: mintLen,
        lamports: lamports,
        programId: TOKEN_2022_PROGRAM_ID,
      }),
      createInitializeTransferHookInstruction(
        mint2.publicKey,
        wallet.publicKey,
        program.programId, // Transfer Hook Program ID
        TOKEN_2022_PROGRAM_ID
      ),
      createInitializeMintInstruction(
        mint2.publicKey,
        decimals,
        wallet.publicKey,
        null,
        TOKEN_2022_PROGRAM_ID
      )
    );

    let txSig = await sendAndConfirmTransaction(
      provider.connection,
      transaction,
      [wallet.payer, mint2]
    );
    console.log(`Transaction Signature: ${txSig}`);
    transaction = new Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: wallet.publicKey,
        newAccountPubkey: mint.publicKey,
        space: mintLen,
        lamports: lamports,
        programId: TOKEN_2022_PROGRAM_ID,
      }),
      createInitializeTransferHookInstruction(
        mint.publicKey,
        wallet.publicKey,
        program.programId, // Transfer Hook Program ID
        TOKEN_2022_PROGRAM_ID
      ),
      createInitializeMintInstruction(
        mint.publicKey,
        decimals,
        wallet.publicKey,
        null,
        TOKEN_2022_PROGRAM_ID
      )
    );

    txSig = await sendAndConfirmTransaction(provider.connection, transaction, [
      wallet.payer,
      mint,
    ]);
    console.log(`Transaction Signature2: ${txSig}`);
  });

  // Create the two token accounts for the transfer-hook enabled mint
  // Fund the sender token account with 100 tokens
  it("Create Token Accounts and Mint Tokens", async () => {
    // 100 tokens
    const amount = 100 * 10 ** decimals;

    const transaction = new Transaction().add(
      createAssociatedTokenAccountInstruction(
        wallet.publicKey,
        sourceTokenAccount,
        wallet.publicKey,
        mint.publicKey,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      ),
      createAssociatedTokenAccountInstruction(
        wallet.publicKey,
        sourceTokenAccount2,
        wallet.publicKey,
        mint2.publicKey,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      ),
      createMintToInstruction(
        mint.publicKey,
        sourceTokenAccount,
        wallet.publicKey,
        amount,
        [],
        TOKEN_2022_PROGRAM_ID
      ),
      createMintToInstruction(
        mint2.publicKey,
        sourceTokenAccount2,
        wallet.publicKey,
        amount,
        [],
        TOKEN_2022_PROGRAM_ID
      )
    );

    const txSig = await sendAndConfirmTransaction(
      connection,
      transaction,
      [wallet.payer],
      { skipPreflight: true }
    );

    console.log(`Transaction Signature: ${txSig}`);
  });

  // Account to store extra accounts required by the transfer hook instruction
  it("Create ExtraAccountMetaList Account", async () => {
    const initializeExtraAccountMetaListInstruction = await program.methods
      .initializeExtraAccountMetaList()
      .accounts({
        payer: wallet.publicKey,
        extraAccountMetaList: extraAccountMetaListPDA1,
        mint: mint.publicKey,
        otherMint: mint2.publicKey,
        game: delegatePDA1,
        systemProgram: SystemProgram.programId,
        raydiumClmm: new PublicKey(
          "8sLbNZoA1cfnvMJLPfp98ZLAnFSYCFApfJKMbiXNLwxj"
        ),
        tokenProgram: TOKEN_2022_PROGRAM_ID,
      })
      .instruction();
    const initializeExtraAccountMetaListInstruction2 = await program.methods
      .initializeExtraAccountMetaList()
      .accounts({
        payer: wallet.publicKey,
        extraAccountMetaList: extraAccountMetaListPDA2,
        mint: mint2.publicKey,
        otherMint: mint.publicKey,
        game: delegatePDA2,
        systemProgram: SystemProgram.programId,
        raydiumClmm: new PublicKey(
          "8sLbNZoA1cfnvMJLPfp98ZLAnFSYCFApfJKMbiXNLwxj"
        ),
        tokenProgram: TOKEN_2022_PROGRAM_ID,
      })
      .instruction();

    const transaction = new Transaction()
      .add(initializeExtraAccountMetaListInstruction)
      .add(initializeExtraAccountMetaListInstruction2);

    const txSig = await sendAndConfirmTransaction(
      provider.connection,
      transaction,
      [wallet.payer],
      { skipPreflight: true, commitment: "confirmed" }
    );
    console.log("Transaction Signature:", txSig);
  });

  it("Transfer Hook with Extra Account Meta", async () => {
    // 1 tokens
    const amount = 1 * 10 ** decimals;
    const bigIntAmount = BigInt(amount);

    // Instruction for sender to fund their WSol token account
    const solTransferInstruction = SystemProgram.transfer({
      fromPubkey: wallet.publicKey,
      toPubkey: senderWSolTokenAccount,
      lamports: amount,
    });

    // Sync sender WSol token account
    const syncWrappedSolInstruction = createSyncNativeInstruction(
      senderWSolTokenAccount
    );
    const recipient = new Keypair();
    const destinationTokenAccount = getAssociatedTokenAddressSync(
      mint.publicKey,
      recipient.publicKey,

      false,
      TOKEN_2022_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );
    // Standard token transfer instruction
    const transferInstruction =
      await createTransferCheckedWithTransferHookInstruction(
        connection,
        sourceTokenAccount,
        mint.publicKey,
        destinationTokenAccount,
        wallet.publicKey,
        bigIntAmount,
        decimals,
        [],
        "confirmed",
        TOKEN_2022_PROGRAM_ID
      );

    console.log("Pushed keys:", JSON.stringify(transferInstruction.keys));

    const transaction = new Transaction().add(transferInstruction);

    const txSig = await sendAndConfirmTransaction(
      connection,
      transaction,
      [wallet.payer],
      { skipPreflight: true }
    );
    console.log("Transfer Signature:", txSig);
  });
});
