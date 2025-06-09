//// No imports needed: web3, anchor, pg and more are globally available

describe("swap-logger", () => {
  const user = pg.wallet;
  const admin = pg.wallet;

  let configPda: web3.PublicKey;
  let userStatePda: web3.PublicKey;
  let tradeRecordPda: web3.PublicKey;
  let bump: number;

  const whitelist = [web3.Keypair.generate().publicKey, web3.Keypair.generate().publicKey];
  const tokenIn = whitelist[0];
  const tokenOut = whitelist[1];

  it("initialize config", async () => {
    [configPda, bump] = await web3.PublicKey.findProgramAddress(
      [Buffer.from("config")],
      pg.program.programId
    );

    const tx = await pg.program.methods
      .initializeConfig(whitelist, 1) // protocol_version = 1
      .accounts({
        config: configPda,
        admin: admin.publicKey,
        systemProgram: web3.SystemProgram.programId,
      })
      .rpc();

    console.log("✅ Config initialized:", tx);
  });

  it("initialize user state", async () => {
    [userStatePda, bump] = await web3.PublicKey.findProgramAddress(
      [Buffer.from("user-state"), user.publicKey.toBuffer()],
      pg.program.programId
    );

    const tx = await pg.program.methods
      .initialize()
      .accounts({
        user: user.publicKey,
        userState: userStatePda,
        systemProgram: web3.SystemProgram.programId,
      })
      .rpc();

    console.log("✅ User state initialized:", tx);
  });

  it("log trade", async () => {
    const userState = await pg.program.account.userState.fetch(userStatePda);
    const tradeCount = new BN(userState.tradeCount);

    [tradeRecordPda, bump] = await web3.PublicKey.findProgramAddress(
      [
        Buffer.from("trade-record"),
        user.publicKey.toBuffer(),
        tradeCount.toArrayLike(Buffer, "le", 8),
      ],
      pg.program.programId
    );

    const tradeType = 0;
    const amount = new BN(1_000_000);
    const price = new BN(500);
    const slippageBps = 50;

    // Properly format a 16-byte tag as number[]
    const tagStr = "SwapTestTag";
    const tagBytes = Array.from(Buffer.from(tagStr.padEnd(16, "\0")));

    const tx = await pg.program.methods
      .logTrade(
        tradeType,
        tokenIn,
        tokenOut,
        amount,
        price,
        slippageBps,
        tagBytes
      )
      .accounts({
        config: configPda,
        userState: userStatePda,
        tradeRecord: tradeRecordPda,
        user: user.publicKey,
        signer: user.publicKey,
        systemProgram: web3.SystemProgram.programId,
      })
      .rpc();

    console.log("✅ Trade logged:", tx);

    const tradeRecord = await pg.program.account.tradeRecord.fetch(tradeRecordPda);
    console.log("🔍 Trade record:", tradeRecord);

    const tagDecoded = Buffer.from(tradeRecord.tag).toString("utf-8").replace(/\0/g, "");
    console.log("📛 Tag (decoded):", tagDecoded);
  });
});
