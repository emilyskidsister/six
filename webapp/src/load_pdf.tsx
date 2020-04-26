import { Song } from "./store";
import { fileOpen } from "browser-nativefs";

async function loadPdf(): Promise<{ error: string } | { song: Song }> {
  const pdf = await fileOpen({
    description: "Six Eight PDF",
    extensions: ["pdf"],
    mimeTypes: ["application/pdf"],
  });
  try {
    const raw = await new Promise<string | ArrayBuffer | null>(
      (resolve, reject) => {
        const reader = new FileReader();
        reader.readAsDataURL(pdf);
        reader.onerror = error => reject(error);
        reader.onload = () => {
          resolve(reader.result);
        };
      },
    );
    const {
      PDFDocument,
      PDFName,
      decodePDFRawStream,
      PDFRawStream,
    } = await import("pdf-lib");
    // @ts-ignore
    const doc = await PDFDocument.load(raw);
    const fileRef = doc?.catalog
      ?.get(PDFName.of("Names"))
      ?.get(PDFName.of("EmbeddedFiles"))
      ?.get(PDFName.of("Names"))
      ?.get(1)
      ?.get(PDFName.of("EF"))
      ?.get(PDFName.of("F"));
    if (!fileRef) {
      return {
        error: "This does not appear to be a PDF generated by Six Eight.",
      };
    }
    const rawStream = doc.context.lookupMaybe(fileRef, PDFRawStream);
    const parsed = decodePDFRawStream(rawStream).decode();
    const song = JSON.parse(new TextDecoder("utf-8").decode(parsed));
    if (song.v !== 1) {
      return { error: "Unsupported song version." };
    }
    return { song };
  } catch (err) {
    console.warn(err);
    return { error: "Failed to process pdf." };
  }
}

export default loadPdf;
