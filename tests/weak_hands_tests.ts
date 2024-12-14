import { Connection, PublicKey } from '@solana/web3.js';
import { expect } from 'chai';
import { describe, it } from 'mocha';

describe('Solana Program Load Test', () => {
  // Configure the connection to devnet
  const connection = new Connection('https://api.devnet.solana.com', 'confirmed');
  
  // The program ID we want to test
  const programId = new PublicKey('DR85urM1zGQhEA5b9MorTjC3FyTacXEgPY9jfmnMt9JX');

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
});