/**
 * Type declarations for snarkjs
 * snarkjs doesn't provide official TypeScript types
 */

declare module 'snarkjs' {
  export namespace groth16 {
    export function fullProve(
      input: any,
      wasmFile: string,
      zkeyFileName: string
    ): Promise<{
      proof: {
        pi_a: string[];
        pi_b: string[][];
        pi_c: string[];
        protocol: string;
        curve: string;
      };
      publicSignals: string[];
    }>;

    export function verify(
      vkey: any,
      publicSignals: string[],
      proof: any
    ): Promise<boolean>;

    export function exportSolidityCallData(
      proof: any,
      publicSignals: string[]
    ): Promise<string>;
  }

  export namespace plonk {
    export function fullProve(
      input: any,
      wasmFile: string,
      zkeyFileName: string
    ): Promise<{
      proof: any;
      publicSignals: string[];
    }>;

    export function verify(
      vkey: any,
      publicSignals: string[],
      proof: any
    ): Promise<boolean>;
  }

  export namespace powersOfTau {
    export function newAccumulator(
      curve: number,
      power: number,
      fileName: string
    ): Promise<void>;

    export function contribute(
      oldPtauFilename: string,
      newPTauFilename: string,
      name: string,
      entropy: string
    ): Promise<void>;
  }

  export namespace zKey {
    export function newZKey(
      r1csName: string,
      ptauName: string,
      zkeyName: string
    ): Promise<void>;

    export function contribute(
      oldZkeyName: string,
      newZKeyName: string,
      name: string,
      entropy: string
    ): Promise<void>;

    export function exportVerificationKey(zkeyName: string): Promise<any>;

    export function exportSolidityVerifier(
      zkeyName: string,
      templates: any
    ): Promise<string>;
  }
}

declare module 'circomlibjs' {
  export function buildPoseidon(): Promise<{
    (inputs: bigint[]): bigint;
    F: {
      toString(value: bigint): string;
    };
  }>;

  export function buildMimc7(): Promise<any>;
  export function buildBabyjub(): Promise<any>;
  export function buildEddsa(): Promise<any>;
}
