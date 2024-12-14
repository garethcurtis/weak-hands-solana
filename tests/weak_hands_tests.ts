import { Connection, PublicKey, Keypair, SystemProgram, Transaction, sendAndConfirmTransaction } from '@solana/web3.js';
import { expect } from 'chai';
import { describe, it } from 'mocha';
import * as anchor from "@project-serum/anchor";
import NodeWallet from '@project-serum/anchor/dist/cjs/nodewallet';

describe('Weak Hands Tests', () => {
  // Configure the connection to devnet
  const connection = new Connection('https://api.devnet.solana.com', 'confirmed');
  
  // The program ID we want to test
  const programId = new PublicKey('DR85urM1zGQhEA5b9MorTjC3FyTacXEgPY9jfmnMt9JX');

  // Set up the wallet - assuming you have a keypair at the default location
  const wallet = new NodeWallet(Keypair.fromSecretKey(
    Buffer.from(JSON.parse(require('fs').readFileSync(
      require('os').homedir() + '/.config/solana/id.json',
      'utf-8'
    )))
  ));

  it('should successfully load the program', async () => {
    try {
      // Attempt to fetch the program account
      const programInfo = await connection.getAccountInfo(programId);
      
      // Check if the program exists
      if (programInfo === null) {
        throw new Error('Program not found on devnet');
      }
      
      // Additional checks to verify it's actually a program
      expect(programInfo.executable).to.be.true;
      expect(programInfo.data.length).to.be.greaterThan(0);
      
    } catch (error) {
      throw error;
    }
  });

  it('can deposit SOL', async () => {
    // Create a new keypair for the lock account
    const lockAccount = Keypair.generate();
    const depositAmount = 1_000_000_000; // 1 SOL in lamports

    try {
      // Get initial balances
      const initialUserBalance = await connection.getBalance(wallet.publicKey);
      const initialLockBalance = await connection.getBalance(lockAccount.publicKey);

      // Create the transaction
      const transaction = new Transaction().add(
        SystemProgram.transfer({
          fromPubkey: wallet.publicKey,
          toPubkey: lockAccount.publicKey,
          lamports: depositAmount,
        })
      );

      // Send and confirm the transaction
      const signature = await sendAndConfirmTransaction(
        connection,
        transaction,
        [wallet.payer]
      );

      // Verify the balances after deposit
      const finalUserBalance = await connection.getBalance(wallet.publicKey);
      const finalLockBalance = await connection.getBalance(lockAccount.publicKey);

      // Account for some SOL being spent on transaction fees
      expect(finalUserBalance).to.be.below(initialUserBalance - depositAmount);
      expect(finalLockBalance).to.equal(initialLockBalance + depositAmount);

    } catch (error) {
      throw error;
    }
  });
});