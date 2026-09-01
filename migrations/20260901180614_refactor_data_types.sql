-- Add new columns
ALTER TABLE books ADD COLUMN public_notes TEXT;
ALTER TABLE books ADD COLUMN dimension_length REAL;
ALTER TABLE books ADD COLUMN dimension_width REAL;
ALTER TABLE books ADD COLUMN dimension_depth REAL;

-- Refactor printing_number
UPDATE books SET printing_number = NULLIF(REGEXP_REPLACE(printing_number, '[^0-9]', '', 'g'), '');
ALTER TABLE books ALTER COLUMN printing_number TYPE INTEGER USING printing_number::integer;

-- Refactor weight
DO $$
DECLARE
    row RECORD;
    w_val REAL;
    w_str TEXT;
BEGIN
    FOR row IN SELECT id, weight FROM books WHERE weight IS NOT NULL LOOP
        w_str := LOWER(row.weight);
        w_val := SUBSTRING(w_str FROM '[0-9]+(?:\.[0-9]+)?')::REAL;
        
        IF w_val IS NOT NULL THEN
            IF w_str LIKE '%kg%' OR w_str LIKE '%kilo%' THEN w_val := w_val * 1000;
            ELSIF w_str LIKE '%oz%' OR w_str LIKE '%ounce%' THEN w_val := w_val * 28.3495;
            ELSIF w_str LIKE '%lb%' OR w_str LIKE '%pound%' THEN w_val := w_val * 453.592;
            END IF;
        END IF;

        UPDATE books SET weight = w_val::TEXT WHERE id = row.id;
    END LOOP;
END $$;

ALTER TABLE books ALTER COLUMN weight TYPE REAL USING weight::real;

-- Refactor dimensions
DO $$
DECLARE
    row RECORD;
    dims TEXT[];
    parsed_length REAL;
    parsed_width REAL;
    parsed_depth REAL;
    multiplier REAL;
    d_str TEXT;
BEGIN
    FOR row IN SELECT id, dimensions FROM books WHERE dimensions IS NOT NULL LOOP
        d_str := LOWER(row.dimensions);
        multiplier := 1.0;
        
        IF d_str LIKE '%in%' THEN multiplier := 2.54;
        ELSIF d_str LIKE '%mm%' THEN multiplier := 0.1;
        END IF;

        SELECT array_agg(m[1]) INTO dims FROM regexp_matches(d_str, '([0-9]+(?:\.[0-9]+)?)', 'g') AS m;
        
        parsed_length := NULL;
        parsed_width := NULL;
        parsed_depth := NULL;
        
        IF array_length(dims, 1) >= 1 THEN parsed_length := dims[1]::REAL * multiplier; END IF;
        IF array_length(dims, 1) >= 2 THEN parsed_width := dims[2]::REAL * multiplier; END IF;
        IF array_length(dims, 1) >= 3 THEN parsed_depth := dims[3]::REAL * multiplier; END IF;

        UPDATE books SET dimension_length = parsed_length, dimension_width = parsed_width, dimension_depth = parsed_depth WHERE id = row.id;
    END LOOP;
END $$;

ALTER TABLE books DROP COLUMN dimensions;
